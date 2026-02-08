mod config;
mod db;
mod error;
mod handlers;
mod middleware;
mod monitor;
mod session;

use actix_web::{middleware as actix_middleware, web, App, HttpServer};
use mongodb::Client;
use std::time::Duration;
use tokio::time;

use config::ServerConfig;
use db::MongoDbContext;
use middleware::{auth_middleware, rate_limit_middleware, RateLimiter};
use monitor::HealthMonitor;
use session::SessionManager;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env file if it exists (for development)
    // Try loading from current directory first, then from server/ directory
    if dotenvy::dotenv().is_err() {
        dotenvy::from_filename("server/.env").ok();
    }

    // Initialize logger
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    log::info!("Starting Connect Server...");
    log::info!("Protocol version: {}", protocol::protocol_version());

    // Load configuration
    let config_path = std::env::var("CONFIG_PATH")
        .unwrap_or_else(|_| "server/config/servers.toml".to_string());

    let config = ServerConfig::load_from_file(&config_path)
        .unwrap_or_else(|e| {
            eprintln!("Failed to load server configuration from '{}': {}", config_path, e);
            eprintln!("Hint: Set CONFIG_PATH environment variable or run from the rust/ directory");
            std::process::exit(1);
        });
    log::info!("Loaded configuration with {} servers", config.servers.len());

    // Connect to MongoDB
    let mongodb_uri =
        std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
    let database_name = std::env::var("DATABASE_NAME").unwrap_or_else(|_| "mu_online".to_string());

    log::info!("Connecting to MongoDB at {}...", mongodb_uri);
    let client = Client::with_uri_str(&mongodb_uri)
        .await
        .expect("Failed to connect to MongoDB");

    let db_context = MongoDbContext::new(client, &database_name);

    // Initialize database indexes
    log::info!("Initializing database indexes...");
    db_context
        .init_indexes()
        .await
        .expect("Failed to initialize database indexes");

    // Create shared state
    let session_expiry_hours = std::env::var("SESSION_EXPIRY_HOURS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24);

    let session_manager = SessionManager::new(session_expiry_hours);
    let health_monitor = HealthMonitor::new();
    let rate_limiter = RateLimiter::new();

    log::info!("Session expiry set to {} hours", session_expiry_hours);

    // Spawn background cleanup tasks
    let session_manager_clone = session_manager.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let removed = session_manager_clone.cleanup_expired();
            if removed > 0 {
                log::info!("Background cleanup: removed {} expired sessions", removed);
            }
        }
    });

    let health_monitor_clone = health_monitor.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let removed = health_monitor_clone.cleanup_stale_heartbeats();
            if removed > 0 {
                log::info!("Background cleanup: marked {} worlds as offline", removed);
            }
        }
    });

    let rate_limiter_clone = rate_limiter.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(300)); // Every 5 minutes
        loop {
            interval.tick().await;
            rate_limiter_clone.cleanup_old_entries();
            log::debug!("Background cleanup: cleaned rate limiter entries");
        }
    });

    // Server configuration
    let server_host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let server_port: u16 = std::env::var("SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    log::info!("Starting HTTP server at {}:{}...", server_host, server_port);

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            // Shared state
            .app_data(web::Data::new(db_context.clone()))
            .app_data(web::Data::new(session_manager.clone()))
            .app_data(web::Data::new(health_monitor.clone()))
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(rate_limiter.clone()))
            // Middleware
            .wrap(actix_middleware::Logger::default())
            .wrap(actix_middleware::Compress::default())
            // Public routes (no authentication required)
            .service(
                web::scope("")
                    .service(handlers::health_check)
                    .service(handlers::heartbeat)
                    .service(handlers::list_servers)
                    .service(handlers::list_worlds)
                    .service(
                        web::scope("")
                            .wrap(actix_middleware::from_fn(rate_limit_middleware))
                            .service(handlers::login),
                    ),
            )
            // Protected routes (authentication required)
            .service(
                web::scope("")
                    .wrap(actix_middleware::from_fn(auth_middleware))
                    .service(handlers::logout)
                    .service(handlers::list_characters),
            )
    })
    .bind((server_host, server_port))?
    .run()
    .await
}
