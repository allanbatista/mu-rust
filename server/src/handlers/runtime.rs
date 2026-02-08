use std::sync::Arc;

use actix_web::{get, web, HttpResponse};
use serde::Serialize;

use crate::{
    error::{ConnectServerError, Result},
    runtime::MuCoreRuntime,
};

fn runtime_ref<'a>(runtime: &'a Option<Arc<MuCoreRuntime>>) -> Result<&'a Arc<MuCoreRuntime>> {
    runtime
        .as_ref()
        .ok_or_else(|| ConnectServerError::Internal("Runtime core is disabled".to_string()))
}

#[derive(Debug, Serialize)]
pub struct RuntimeWorldsResponse {
    pub worlds: crate::runtime::directory::WorldDirectorySnapshot,
}

#[derive(Debug, Serialize)]
pub struct RuntimeMapsResponse {
    pub maps: Vec<crate::runtime::map_server::MapServerStats>,
}

#[derive(Debug, Serialize)]
pub struct RuntimePersistenceResponse {
    pub metrics: crate::runtime::persistence::PersistenceMetrics,
}

#[derive(Debug, Serialize)]
pub struct RuntimeStatsResponse {
    pub stats: crate::runtime::core::RuntimeStats,
}

#[get("/runtime/worlds")]
pub async fn runtime_worlds(
    runtime: web::Data<Option<Arc<MuCoreRuntime>>>,
) -> Result<HttpResponse> {
    let runtime = runtime_ref(runtime.get_ref())?;
    let response = RuntimeWorldsResponse {
        worlds: runtime.directory_snapshot(),
    };
    Ok(HttpResponse::Ok().json(response))
}

#[get("/runtime/maps")]
pub async fn runtime_maps(runtime: web::Data<Option<Arc<MuCoreRuntime>>>) -> Result<HttpResponse> {
    let runtime = runtime_ref(runtime.get_ref())?;
    let maps = runtime.map_stats().await;
    Ok(HttpResponse::Ok().json(RuntimeMapsResponse { maps }))
}

#[get("/runtime/persistence")]
pub async fn runtime_persistence(
    runtime: web::Data<Option<Arc<MuCoreRuntime>>>,
) -> Result<HttpResponse> {
    let runtime = runtime_ref(runtime.get_ref())?;
    let metrics = runtime.persistence_metrics().await;
    Ok(HttpResponse::Ok().json(RuntimePersistenceResponse { metrics }))
}

#[get("/runtime/stats")]
pub async fn runtime_stats(runtime: web::Data<Option<Arc<MuCoreRuntime>>>) -> Result<HttpResponse> {
    let runtime = runtime_ref(runtime.get_ref())?;
    let stats = runtime.runtime_stats().await;
    Ok(HttpResponse::Ok().json(RuntimeStatsResponse { stats }))
}
