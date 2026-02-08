use actix_web::{test, web, App};
use server::handlers;
use server::monitor::HealthMonitor;

#[actix_web::test]
async fn test_health_check() {
    let health_monitor = HealthMonitor::new();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(health_monitor.clone()))
            .service(handlers::health_check),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "healthy");
    assert!(body.get("timestamp").is_some());
}

#[actix_web::test]
async fn test_health_check_with_active_worlds() {
    let health_monitor = HealthMonitor::new();

    // Add a test world
    health_monitor.record_heartbeat("test-world-1".to_string(), 10);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(health_monitor.clone()))
            .service(handlers::health_check),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["active_worlds"], 1);
}
