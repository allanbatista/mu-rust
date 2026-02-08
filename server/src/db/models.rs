use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub username: String,
    pub password_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_login: DateTime<Utc>,
}

impl Account {
    pub fn new(username: String, password: &str) -> Result<Self> {
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;

        Ok(Self {
            id: None,
            username,
            password_hash,
            email: None,
            created_at: Utc::now(),
            last_login: Utc::now(),
        })
    }

    pub fn verify_password(&self, password: &str) -> Result<bool> {
        Ok(bcrypt::verify(password, &self.password_hash)?)
    }

    pub fn update_last_login(&mut self) {
        self.last_login = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub account_id: ObjectId,
    pub name: String,
    pub level: u16,
    pub class: String,
    pub created_at: DateTime<Utc>,
}

impl Character {
    pub fn new(account_id: ObjectId, name: String, class: String) -> Self {
        Self {
            id: None,
            account_id,
            name,
            level: 1,
            class,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_new() {
        let account = Account::new("testuser".to_string(), "password123").unwrap();
        assert_eq!(account.username, "testuser");
        assert_ne!(account.password_hash, "password123");
        assert!(account.id.is_none());
    }

    #[test]
    fn test_verify_password_correct() {
        let account = Account::new("testuser".to_string(), "password123").unwrap();
        assert!(account.verify_password("password123").unwrap());
    }

    #[test]
    fn test_verify_password_incorrect() {
        let account = Account::new("testuser".to_string(), "password123").unwrap();
        assert!(!account.verify_password("wrongpassword").unwrap());
    }

    #[test]
    fn test_character_new() {
        let account_id = ObjectId::new();
        let character =
            Character::new(account_id, "TestChar".to_string(), "DarkKnight".to_string());

        assert_eq!(character.name, "TestChar");
        assert_eq!(character.class, "DarkKnight");
        assert_eq!(character.level, 1);
        assert_eq!(character.account_id, account_id);
    }
}
