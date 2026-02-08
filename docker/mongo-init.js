// MongoDB initialization script for Mu Online project
// This script runs automatically when the container is first created

db = db.getSiblingDB('mu');

// Create collections
db.createCollection('users');
db.createCollection('characters');
db.createCollection('guilds');
db.createCollection('items');
db.createCollection('game_sessions');

// Create indexes for users collection
db.users.createIndex({ "username": 1 }, { unique: true });
db.users.createIndex({ "email": 1 }, { unique: true });
db.users.createIndex({ "created_at": 1 });

// Create indexes for characters collection
db.characters.createIndex({ "user_id": 1 });
db.characters.createIndex({ "name": 1 }, { unique: true });
db.characters.createIndex({ "level": -1 });
db.characters.createIndex({ "class": 1 });

// Create indexes for guilds collection
db.guilds.createIndex({ "name": 1 }, { unique: true });
db.guilds.createIndex({ "master_id": 1 });

// Create indexes for items collection
db.items.createIndex({ "character_id": 1 });
db.items.createIndex({ "item_type": 1 });

// Create indexes for game_sessions collection
db.game_sessions.createIndex({ "user_id": 1 });
db.game_sessions.createIndex({ "session_token": 1 }, { unique: true });
db.game_sessions.createIndex({ "created_at": 1 }, { expireAfterSeconds: 86400 }); // 24 hours TTL

print('✅ Mu database initialized successfully');
print('✅ Collections created: users, characters, guilds, items, game_sessions');
print('✅ Indexes created for optimal query performance');
