use mongodb::{
    bson::{doc, oid::ObjectId, DateTime as BsonDateTime},
    Client, Collection, Database,
};

use super::models::{Account, Character};
use crate::error::Result;

#[derive(Clone)]
pub struct MongoDbContext {
    db: Database,
}

impl MongoDbContext {
    pub fn new(client: Client, database_name: &str) -> Self {
        Self {
            db: client.database(database_name),
        }
    }

    pub fn accounts(&self) -> AccountRepository {
        AccountRepository {
            collection: self.db.collection("accounts"),
        }
    }

    pub fn characters(&self) -> CharacterRepository {
        CharacterRepository {
            collection: self.db.collection("characters"),
        }
    }

    pub async fn init_indexes(&self) -> Result<()> {
        use mongodb::options::IndexOptions;
        use mongodb::IndexModel;

        // Create unique index on username
        let username_index = IndexModel::builder()
            .keys(doc! { "username": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build();

        self.db
            .collection::<Account>("accounts")
            .create_index(username_index)
            .await?;

        // Create index on account_id for characters
        let account_index = IndexModel::builder().keys(doc! { "account_id": 1 }).build();

        self.db
            .collection::<Character>("characters")
            .create_index(account_index)
            .await?;

        // Create unique index on character name
        let character_name_index = IndexModel::builder()
            .keys(doc! { "name": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build();

        self.db
            .collection::<Character>("characters")
            .create_index(character_name_index)
            .await?;

        log::info!("Database indexes created successfully");
        Ok(())
    }
}

#[derive(Clone)]
pub struct AccountRepository {
    collection: Collection<Account>,
}

impl AccountRepository {
    pub async fn find_by_username(&self, username: &str) -> Result<Option<Account>> {
        let account = self
            .collection
            .find_one(doc! { "username": username })
            .await?;
        Ok(account)
    }

    pub async fn update_last_login(&self, id: &ObjectId) -> Result<()> {
        let now = BsonDateTime::now();
        self.collection
            .update_one(doc! { "_id": id }, doc! { "$set": { "last_login": now } })
            .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct CharacterRepository {
    collection: Collection<Character>,
}

impl CharacterRepository {
    pub async fn find_by_account_id(&self, account_id: &ObjectId) -> Result<Vec<Character>> {
        let mut cursor = self
            .collection
            .find(doc! { "account_id": account_id })
            .await?;

        let mut characters = Vec::new();
        use futures_util::stream::TryStreamExt;
        while let Some(character) = cursor.try_next().await? {
            characters.push(character);
        }

        Ok(characters)
    }
}
