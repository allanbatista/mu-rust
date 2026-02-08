use actix_web::{cookie::Cookie, post, web, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::{
    db::MongoDbContext,
    error::{ConnectServerError, Result},
    session::SessionManager,
};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub account_id: String,
    pub message: String,
}

#[post("/login")]
pub async fn login(
    req: web::Json<LoginRequest>,
    db: web::Data<MongoDbContext>,
    session_manager: web::Data<SessionManager>,
) -> Result<HttpResponse> {
    log::info!("Login attempt for user: {}", req.username);

    // Find account by username
    let account = db
        .accounts()
        .find_by_username(&req.username)
        .await?
        .ok_or(ConnectServerError::InvalidCredentials)?;

    // Verify password
    if !account.verify_password(&req.password)? {
        log::warn!("Failed login attempt for user: {}", req.username);
        return Err(ConnectServerError::InvalidCredentials);
    }

    let account_id = account.id.expect("Account should have ID");

    // Create session (will kick old session if exists)
    let session = session_manager.create_session(account_id)?;

    // Update last login time
    db.accounts().update_last_login(&account_id).await?;

    log::info!(
        "Successful login for user: {} (session: {})",
        req.username,
        session.session_id
    );

    // Create session cookie
    let cookie = Cookie::build("session_id", session.session_id.clone())
        .path("/")
        .http_only(true)
        .same_site(actix_web::cookie::SameSite::Strict)
        .max_age(actix_web::cookie::time::Duration::hours(24))
        .finish();

    let response = LoginResponse {
        success: true,
        account_id: account_id.to_hex(),
        message: "Login successful".to_string(),
    };

    Ok(HttpResponse::Ok().cookie(cookie).json(response))
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub success: bool,
    pub message: String,
}

#[post("/logout")]
pub async fn logout(
    session_manager: web::Data<SessionManager>,
    session_id: web::ReqData<String>,
) -> Result<HttpResponse> {
    let session_id = session_id.into_inner();
    session_manager.invalidate_session(&session_id);

    log::info!("User logged out (session: {})", session_id);

    let cookie = Cookie::build("session_id", "")
        .path("/")
        .max_age(actix_web::cookie::time::Duration::seconds(0))
        .finish();

    let response = LogoutResponse {
        success: true,
        message: "Logout successful".to_string(),
    };

    Ok(HttpResponse::Ok().cookie(cookie).json(response))
}
