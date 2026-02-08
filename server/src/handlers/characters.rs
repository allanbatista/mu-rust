use actix_web::{get, web, HttpResponse};
use serde::Serialize;

use crate::{db::MongoDbContext, error::Result, session::SessionManager};

#[derive(Debug, Serialize)]
pub struct CharacterListResponse {
    pub characters: Vec<CharacterInfo>,
}

#[derive(Debug, Serialize)]
pub struct CharacterInfo {
    pub id: String,
    pub name: String,
    pub level: u16,
    pub class: String,
}

#[get("/characters")]
pub async fn list_characters(
    db: web::Data<MongoDbContext>,
    session_manager: web::Data<SessionManager>,
    session_id: web::ReqData<String>,
) -> Result<HttpResponse> {
    let session_id = session_id.into_inner();

    // Validate session
    let session = session_manager.validate_session(&session_id)?;

    // Get characters for this account
    let characters = db
        .characters()
        .find_by_account_id(&session.account_id)
        .await?;

    let character_list: Vec<CharacterInfo> = characters
        .iter()
        .map(|c| CharacterInfo {
            id: c.id.expect("Character should have ID").to_hex(),
            name: c.name.clone(),
            level: c.level,
            class: c.class.clone(),
        })
        .collect();

    let count = character_list.len();

    let response = CharacterListResponse {
        characters: character_list,
    };

    log::info!(
        "Listed {} characters for account {}",
        count,
        session.account_id.to_hex()
    );

    Ok(HttpResponse::Ok().json(response))
}
