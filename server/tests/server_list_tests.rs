use actix_web::{test, web, App};
use server::config::ServerConfig;
use server::handlers;
use server::monitor::HealthMonitor;

fn servers_config_path() -> String {
    format!("{}/config/servers.toml", env!("CARGO_MANIFEST_DIR"))
}

#[actix_web::test]
async fn test_list_servers() {
    let config =
        ServerConfig::load_from_file(servers_config_path()).expect("Failed to load test config");

    let health_monitor = HealthMonitor::new();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(health_monitor.clone()))
            .service(handlers::list_servers),
    )
    .await;

    let req = test::TestRequest::get().uri("/servers").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["servers"].is_array());
    assert_eq!(body["servers"].as_array().unwrap().len(), 2);
    assert!(body["servers"][0]["id"].is_string());
}

#[actix_web::test]
async fn test_list_worlds_only_online() {
    let config =
        ServerConfig::load_from_file(servers_config_path()).expect("Failed to load test config");

    let health_monitor = HealthMonitor::new();

    // Simulate some worlds being online
    health_monitor.record_heartbeat("world-1-lorencia".to_string(), 50);
    health_monitor.record_heartbeat("world-1-noria".to_string(), 75);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(health_monitor.clone()))
            .service(handlers::list_worlds),
    )
    .await;

    let req = test::TestRequest::get().uri("/worlds").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["worlds"].is_array());
    assert_eq!(body["worlds"].as_array().unwrap().len(), 2);
}

#[actix_web::test]
async fn test_heartbeat_endpoint_updates_monitor() {
    use serde_json::json;

    let health_monitor = HealthMonitor::new();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(health_monitor.clone()))
            .service(handlers::heartbeat),
    )
    .await;

    let payload = json!({
        "world_id": "test-world",
        "current_players": 42,
        "timestamp": 123456
    });

    let req = test::TestRequest::post()
        .uri("/heartbeat")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["next_heartbeat_in"], 15);

    assert!(health_monitor.is_world_online("test-world"));
}
