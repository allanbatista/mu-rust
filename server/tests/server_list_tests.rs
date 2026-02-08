use actix_web::{test, web, App};
use server::config::ServerConfig;
use server::handlers;
use server::monitor::HealthMonitor;

#[actix_web::test]
async fn test_list_servers() {
    // Load test configuration
    let config = ServerConfig::load_from_file("server/config/servers.toml")
        .expect("Failed to load test config");

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
}

#[actix_web::test]
async fn test_list_worlds() {
    let config = ServerConfig::load_from_file("server/config/servers.toml")
        .expect("Failed to load test config");

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
    assert!(body["servers"].is_array());

    // Count online worlds
    let mut total_online = 0;
    for server in body["servers"].as_array().unwrap() {
        total_online += server["worlds"].as_array().unwrap().len();
    }
    assert_eq!(total_online, 2);
}

#[actix_web::test]
async fn test_heartbeat() {
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
        "ip": "192.168.1.100",
        "port": 55901
    });

    let req = test::TestRequest::post()
        .uri("/heartbeat")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");

    // Verify the world is now tracked
    assert!(health_monitor.is_world_online("test-world"));
}
