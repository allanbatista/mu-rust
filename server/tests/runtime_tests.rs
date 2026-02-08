use std::sync::Arc;
use std::time::Duration;

use actix_web::{test, web, App};
use server::auth_token::AuthTokenService;
use server::handlers;
use server::runtime::{MuCoreRuntime, RuntimeConfig};

#[actix_web::test]
async fn runtime_endpoints_return_data_when_enabled() {
    let auth_tokens = AuthTokenService::new(
        b"01234567890123456789012345678901".to_vec(),
        Duration::from_secs(3600),
    )
    .expect("auth tokens");
    let runtime = Arc::new(
        MuCoreRuntime::bootstrap(RuntimeConfig::default(), auth_tokens).expect("runtime"),
    );

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Some(runtime.clone())))
            .service(handlers::runtime_worlds)
            .service(handlers::runtime_maps)
            .service(handlers::runtime_persistence)
            .service(handlers::runtime_stats),
    )
    .await;

    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/runtime/worlds").to_request(),
    )
    .await;
    assert!(resp.status().is_success());
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["worlds"]["worlds"].is_array());

    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/runtime/maps").to_request(),
    )
    .await;
    assert!(resp.status().is_success());
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["maps"].is_array());

    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/runtime/persistence")
            .to_request(),
    )
    .await;
    assert!(resp.status().is_success());
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["metrics"].is_object());

    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/runtime/stats").to_request(),
    )
    .await;
    assert!(resp.status().is_success());
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["stats"]["online_maps"].is_number());

    runtime.shutdown().await.unwrap();
}

#[actix_web::test]
async fn runtime_endpoints_fail_when_disabled() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(None::<Arc<MuCoreRuntime>>))
            .service(handlers::runtime_stats),
    )
    .await;

    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/runtime/stats").to_request(),
    )
    .await;

    assert_eq!(
        resp.status(),
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
    );
}
