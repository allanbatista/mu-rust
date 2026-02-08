use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorUnauthorized,
    middleware::Next,
    HttpMessage,
};

use crate::session::SessionManager;

pub async fn auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    // Extract session_id from cookie
    let session_id = req
        .cookie("session_id")
        .map(|c| c.value().to_string())
        .ok_or_else(|| ErrorUnauthorized("Authentication required"))?;

    // Get SessionManager from app data
    let session_manager = req
        .app_data::<actix_web::web::Data<SessionManager>>()
        .ok_or_else(|| ErrorUnauthorized("Session manager not available"))?;

    // Validate session
    session_manager
        .validate_session(&session_id)
        .map_err(|_| ErrorUnauthorized("Invalid or expired session"))?;

    // Store session_id in request extensions for handlers to use
    req.extensions_mut().insert(session_id.clone());

    // Also make it available via ReqData
    req.extensions_mut().insert(session_id);

    next.call(req).await
}
