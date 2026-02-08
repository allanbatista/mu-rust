use dashmap::DashMap;
use mongodb::bson::oid::ObjectId;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::error::{ConnectServerError, Result};

#[derive(Debug, Clone)]
pub struct SessionData {
    pub session_id: String,
    pub account_id: ObjectId,
    pub character_id: Option<ObjectId>,
    pub expires_at: Instant,
}

impl SessionData {
    pub fn new(account_id: ObjectId, expiry_hours: u64) -> Self {
        let session_id = Uuid::new_v4().to_string();
        let now = Instant::now();
        let expires_at = now + Duration::from_secs(expiry_hours * 3600);

        Self {
            session_id: session_id.clone(),
            account_id,
            character_id: None,
            expires_at,
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

#[derive(Clone)]
pub struct SessionManager {
    // session_id -> SessionData
    sessions: Arc<DashMap<String, SessionData>>,
    // account_id -> session_id
    account_sessions: Arc<DashMap<String, String>>,
    // character_id -> session_id
    character_sessions: Arc<DashMap<String, String>>,
    expiry_hours: u64,
}

impl SessionManager {
    pub fn new(expiry_hours: u64) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            account_sessions: Arc::new(DashMap::new()),
            character_sessions: Arc::new(DashMap::new()),
            expiry_hours,
        }
    }

    pub fn create_session(&self, account_id: ObjectId) -> Result<SessionData> {
        let account_id_str = account_id.to_hex();

        // Check for existing session (duplicate login)
        let old_session_id = self
            .account_sessions
            .get(&account_id_str)
            .map(|entry| entry.value().clone());
        if let Some(old_session_id) = old_session_id {
            // Invalidate old session
            self.invalidate_session(&old_session_id);
            log::info!("Kicked old session for account: {}", account_id_str);
        }

        let session_data = SessionData::new(account_id, self.expiry_hours);

        self.sessions
            .insert(session_data.session_id.clone(), session_data.clone());
        self.account_sessions
            .insert(account_id_str, session_data.session_id.clone());

        log::info!(
            "Created session {} for account {}",
            session_data.session_id,
            account_id.to_hex()
        );

        Ok(session_data)
    }

    pub fn validate_session(&self, session_id: &str) -> Result<SessionData> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or(ConnectServerError::InvalidSession)?;

        if session.is_expired() {
            drop(session);
            self.invalidate_session(session_id);
            return Err(ConnectServerError::InvalidSession);
        }

        Ok(session.clone())
    }

    #[cfg(test)]
    pub fn get_session(&self, session_id: &str) -> Option<SessionData> {
        self.sessions.get(session_id).map(|s| s.clone())
    }

    pub fn invalidate_session(&self, session_id: &str) {
        if let Some((_, session)) = self.sessions.remove(session_id) {
            let account_id_str = session.account_id.to_hex();
            self.account_sessions.remove(&account_id_str);

            if let Some(character_id) = session.character_id {
                let character_id_str = character_id.to_hex();
                self.character_sessions.remove(&character_id_str);
            }

            log::info!("Invalidated session: {}", session_id);
        }
    }

    #[cfg(test)]
    pub fn select_character(&self, session_id: &str, character_id: ObjectId) -> Result<()> {
        let character_id_str = character_id.to_hex();

        // Check if character is already in use
        if let Some(existing_session_id) = self.character_sessions.get(&character_id_str) {
            if existing_session_id.as_str() != session_id {
                return Err(ConnectServerError::Internal(
                    "Character already in use".to_string(),
                ));
            }
        }

        // Update session with character
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            // Remove old character binding if exists
            if let Some(old_char_id) = session.character_id {
                self.character_sessions.remove(&old_char_id.to_hex());
            }

            session.character_id = Some(character_id);
            self.character_sessions
                .insert(character_id_str, session_id.to_string());

            log::info!(
                "Character {} selected for session {}",
                character_id.to_hex(),
                session_id
            );
            Ok(())
        } else {
            Err(ConnectServerError::InvalidSession)
        }
    }

    pub fn cleanup_expired(&self) -> usize {
        let mut removed = 0;

        self.sessions.retain(|session_id, session| {
            if session.is_expired() {
                let account_id_str = session.account_id.to_hex();
                self.account_sessions.remove(&account_id_str);

                if let Some(character_id) = session.character_id {
                    self.character_sessions.remove(&character_id.to_hex());
                }

                log::debug!("Cleaned up expired session: {}", session_id);
                removed += 1;
                false
            } else {
                true
            }
        });

        if removed > 0 {
            log::info!("Cleaned up {} expired sessions", removed);
        }

        removed
    }

    pub fn active_session_count(&self) -> usize {
        self.sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let manager = SessionManager::new(24);
        let account_id = ObjectId::new();

        let session = manager.create_session(account_id).unwrap();
        assert_eq!(session.account_id, account_id);
        assert_eq!(manager.active_session_count(), 1);
    }

    #[test]
    fn test_validate_session() {
        let manager = SessionManager::new(24);
        let account_id = ObjectId::new();

        let session = manager.create_session(account_id).unwrap();
        let validated = manager.validate_session(&session.session_id).unwrap();

        assert_eq!(validated.account_id, account_id);
    }

    #[test]
    fn test_invalidate_session() {
        let manager = SessionManager::new(24);
        let account_id = ObjectId::new();

        let session = manager.create_session(account_id).unwrap();
        assert_eq!(manager.active_session_count(), 1);

        manager.invalidate_session(&session.session_id);
        assert_eq!(manager.active_session_count(), 0);
    }

    #[test]
    fn test_duplicate_login_kicks_old_session() {
        let manager = SessionManager::new(24);
        let account_id = ObjectId::new();

        let session1 = manager.create_session(account_id).unwrap();
        assert_eq!(manager.active_session_count(), 1);

        let session2 = manager.create_session(account_id).unwrap();
        assert_eq!(manager.active_session_count(), 1);
        assert_ne!(session1.session_id, session2.session_id);

        // Old session should be invalid
        assert!(manager.validate_session(&session1.session_id).is_err());
        // New session should be valid
        assert!(manager.validate_session(&session2.session_id).is_ok());
    }

    #[test]
    fn test_select_character() {
        let manager = SessionManager::new(24);
        let account_id = ObjectId::new();
        let character_id = ObjectId::new();

        let session = manager.create_session(account_id).unwrap();
        manager
            .select_character(&session.session_id, character_id)
            .unwrap();

        let updated = manager.get_session(&session.session_id).unwrap();
        assert_eq!(updated.character_id, Some(character_id));
    }

    #[test]
    fn test_session_expiry() {
        let manager = SessionManager::new(0); // Expire immediately
        let account_id = ObjectId::new();

        let session = manager.create_session(account_id).unwrap();

        // Wait a bit to ensure expiry
        std::thread::sleep(Duration::from_millis(10));

        // Session should be expired
        assert!(manager.validate_session(&session.session_id).is_err());
    }
}
