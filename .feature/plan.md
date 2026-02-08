# Feature: Connect Server - Microservices Authentication & Discovery Service

## 📋 Executive Summary
- **Objective**: Implement a standalone Connect Server as the first microservice in the MU Online architecture, responsible for user authentication, session management, and game server discovery
- **Scope**: HTTP REST API with JSON responses, MongoDB persistence, in-memory session management, and heartbeat-based health monitoring
- **Estimated Effort**: 3-5 days (depending on testing thoroughness)
- **Risk Level**: Medium
- **Dependencies**:
  - `actix-web` (HTTP framework)
  - `mongodb` driver + potential ORM
  - `bcrypt` (password hashing)
  - `serde` (JSON serialization)
  - `tokio` (async runtime)
  - Configuration management crate (`config` or `figment`)

## 🔍 Requirements Analysis

### Functional Requirements

1. **User Authentication**
   - Accept username/password via `POST /login`
   - Hash passwords using bcrypt
   - Return session cookie on successful authentication
   - Validate credentials against MongoDB
   - **Acceptance Criteria**:
     - [ ] Returns 200 + session cookie on valid credentials
     - [ ] Returns 401 on invalid credentials
     - [ ] Returns 409 if user already logged in (kicks old session)
     - [ ] Session persists across requests

2. **Session Management**
   - Store active sessions in-memory (HashMap)
   - Track account + character per session
   - Prevent duplicate logins (kick old session)
   - **Acceptance Criteria**:
     - [ ] Only one session per account allowed
     - [ ] Only one session per character allowed
     - [ ] Old session invalidated when new login occurs
     - [ ] Session data includes account ID, character ID, login timestamp

3. **Server Listing**
   - Load server configurations from file (TOML/YAML)
   - Return list via `GET /servers`
   - Include server name, description, status
   - **Acceptance Criteria**:
     - [ ] Returns all configured servers
     - [ ] Shows online/offline status based on heartbeat
     - [ ] JSON response format matches API spec

4. **World/Level Server Discovery**
   - List available world servers via `GET /worlds`
   - Each world represents a game level/map instance
   - Include connection details (IP, port)
   - Filter by online status
   - **Acceptance Criteria**:
     - [ ] Returns only healthy (heartbeat-responsive) servers
     - [ ] Includes IP, port, world/level name
     - [ ] Response indicates current player count

5. **Character Listing**
   - Return user's characters via `GET /characters` (authenticated endpoint)
   - Requires valid session cookie
   - **Acceptance Criteria**:
     - [ ] Returns 401 if not authenticated
     - [ ] Returns character list from MongoDB
     - [ ] Includes character name, level, class

6. **Health Check System**
   - Game servers send periodic heartbeats
   - Mark servers as offline if heartbeat timeout exceeded
   - Background task monitors heartbeat status
   - **Acceptance Criteria**:
     - [ ] Heartbeat endpoint accepts server registration
     - [ ] Servers marked offline after 30s without heartbeat
     - [ ] Status reflected in `/servers` and `/worlds` responses

### Non-Functional Requirements
- **Performance**:
  - Handle 1000 concurrent users
  - Login endpoint < 200ms p95 latency
  - Session lookup < 10ms
- **Security**:
  - Bcrypt password hashing (cost factor 12)
  - Secure session cookies (HttpOnly, SameSite)
  - No password logging
  - Rate limiting on login endpoint (10 req/min per IP)
- **Scalability**:
  - Stateless design (sessions in-memory acceptable for MVP, Redis migration path documented)
  - Horizontal scaling ready (when sessions moved to Redis)
- **Reliability**:
  - Graceful MongoDB connection handling
  - Health check endpoint for load balancer
  - Structured logging for debugging

### User Stories

#### Story 1: User Login
**As a** player
**I want to** authenticate with my username and password
**So that** I can access my game account and characters
- **Acceptance Criteria**:
  - [ ] Send `POST /login` with `{"username": "...", "password": "..."}`
  - [ ] Receive session cookie on success
  - [ ] Session cookie allows access to protected endpoints
  - [ ] Old session is kicked if I login from another device

#### Story 2: Choose Game Server
**As a** player
**I want to** see a list of available game servers
**So that** I can choose which server to play on
- **Acceptance Criteria**:
  - [ ] Call `GET /servers` to see server list
  - [ ] See server name, description, online status
  - [ ] Only see servers that are currently online

#### Story 3: Choose World/Level
**As a** player
**I want to** see which world servers are available
**So that** I can connect to the appropriate game instance for my level
- **Acceptance Criteria**:
  - [ ] Call `GET /worlds` to see world instances
  - [ ] See IP, port, world name, player count
  - [ ] Client can use this info to establish game connection

#### Story 4: View Characters
**As an** authenticated player
**I want to** see my character list
**So that** I can choose which character to play
- **Acceptance Criteria**:
  - [ ] Call `GET /characters` (requires session cookie)
  - [ ] See character name, level, class
  - [ ] Receive error if not authenticated

## 🏗️ Technical Architecture

### Component Design

#### Backend Components

1. **HTTP Server (`actix-web`)**
   - Main application entry point
   - Route handlers for REST endpoints
   - Middleware: logging, session validation, rate limiting
   - Error handling and response formatting

2. **Session Manager (In-Memory)**
   - `HashMap<SessionId, SessionData>` protected by `RwLock`
   - `HashMap<AccountId, SessionId>` for duplicate detection
   - `HashMap<CharacterId, SessionId>` for character locking
   - Background cleanup task for expired sessions

3. **Database Layer (MongoDB)**
   - Collections:
     - `accounts`: User credentials and metadata
     - `characters`: Character data (name, level, class, owner)
   - Use `mongodb` crate directly or ORM if available (e.g., `mongod`)
   - Connection pool management

4. **Configuration Manager**
   - Load server list from `config/servers.toml`
   - Parse game server definitions (name, IP, port, world)
   - Hot-reload capability (optional, phase 2)

5. **Health Monitor**
   - Background task (Tokio task spawned on startup)
   - Maintains `HashMap<ServerId, LastHeartbeat>`
   - Marks servers offline after 30s timeout
   - Provides status query interface

#### Data Models

**MongoDB Schema**:
```json
// accounts collection
{
  "_id": ObjectId,
  "username": String (unique index),
  "password_hash": String (bcrypt),
  "email": String (optional),
  "created_at": DateTime,
  "last_login": DateTime
}

// characters collection
{
  "_id": ObjectId,
  "account_id": ObjectId (indexed),
  "name": String (unique index),
  "level": u16,
  "class": String,
  "created_at": DateTime
}
```

**In-Memory Session**:
```rust
struct SessionData {
    session_id: String,
    account_id: ObjectId,
    character_id: Option<ObjectId>,
    login_timestamp: Instant,
    expires_at: Instant,
}
```

**Server Configuration**:
```toml
# config/servers.toml
[[servers]]
id = "server-1"
name = "Alpha Server"
description = "Main game server"

[[servers.worlds]]
id = "world-1-lorencia"
name = "Lorencia"
ip = "127.0.0.1"
port = 55901
max_players = 100
```

### API Specification

#### Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/login` | No | Authenticate user |
| POST | `/logout` | Yes | Invalidate session |
| GET | `/servers` | No | List game servers |
| GET | `/worlds` | No | List world instances |
| GET | `/characters` | Yes | List user's characters |
| POST | `/heartbeat` | No | Game server health check |
| GET | `/health` | No | Connect server health |

#### Request/Response Examples

**POST /login**
```json
// Request
{
  "username": "player1",
  "password": "secret123"
}

// Response 200
{
  "success": true,
  "account_id": "507f1f77bcf86cd799439011",
  "message": "Login successful"
}
// + Set-Cookie: session_id=abc123; HttpOnly; SameSite=Strict; Max-Age=86400

// Response 401
{
  "success": false,
  "error": "Invalid credentials"
}

// Response 409
{
  "success": true,
  "message": "Previous session terminated",
  "account_id": "507f1f77bcf86cd799439011"
}
```

**GET /servers**
```json
// Response 200
{
  "servers": [
    {
      "id": "server-1",
      "name": "Alpha Server",
      "description": "Main game server",
      "status": "online",
      "world_count": 3
    }
  ]
}
```

**GET /worlds**
```json
// Response 200
{
  "worlds": [
    {
      "id": "world-1-lorencia",
      "name": "Lorencia",
      "server_id": "server-1",
      "ip": "127.0.0.1",
      "port": 55901,
      "status": "online",
      "current_players": 42,
      "max_players": 100
    }
  ]
}
```

**GET /characters**
```json
// Response 200 (authenticated)
{
  "characters": [
    {
      "id": "507f1f77bcf86cd799439012",
      "name": "WarriorX",
      "level": 150,
      "class": "DarkKnight"
    }
  ]
}

// Response 401 (not authenticated)
{
  "success": false,
  "error": "Authentication required"
}
```

**POST /heartbeat**
```json
// Request
{
  "world_id": "world-1-lorencia",
  "current_players": 42,
  "timestamp": 1634567890
}

// Response 200
{
  "success": true,
  "next_heartbeat_in": 15
}
```

### Data Flow

```
Client → POST /login → ConnectServer → MongoDB (validate) → SessionManager (create) → Response + Cookie
Client → GET /characters (+ cookie) → SessionManager (validate) → MongoDB (query) → Response
GameServer → POST /heartbeat → HealthMonitor (update timestamp) → Response
Client → GET /worlds → HealthMonitor (query status) → ConfigManager (server list) → Response
```

## 📁 Implementation Plan

### Project Structure
```
rust/
├── server/
│   ├── Cargo.toml (updated dependencies)
│   ├── src/
│   │   ├── main.rs (application entry point)
│   │   ├── config/
│   │   │   ├── mod.rs
│   │   │   └── server_config.rs (load servers.toml)
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   ├── models.rs (Account, Character structs)
│   │   │   └── repository.rs (MongoDB operations)
│   │   ├── handlers/
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs (login, logout)
│   │   │   ├── servers.rs (list servers, worlds)
│   │   │   ├── characters.rs (list characters)
│   │   │   └── health.rs (heartbeat, health check)
│   │   ├── middleware/
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs (session validation)
│   │   │   └── rate_limit.rs (login rate limiting)
│   │   ├── session/
│   │   │   ├── mod.rs
│   │   │   └── manager.rs (in-memory session store)
│   │   ├── monitor/
│   │   │   ├── mod.rs
│   │   │   └── health.rs (heartbeat monitor task)
│   │   └── error.rs (custom error types)
│   └── config/
│       └── servers.toml (server configuration)
└── protocol/ (keep existing, may add new JSON models)
```

### Detailed Implementation Steps

#### Phase 1: Project Setup & Dependencies (Day 1)

**1.1 Update server/Cargo.toml**
- **File**: `rust/server/Cargo.toml`
- **Purpose**: Add all required dependencies
- **Changes**:
  ```toml
  [dependencies]
  actix-web = "4"
  actix-rt = "2"
  tokio = { version = "1", features = ["full"] }
  mongodb = "3"
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  bcrypt = "0.15"
  config = "0.14"
  toml = "0.8"
  chrono = { version = "0.4", features = ["serde"] }
  uuid = { version = "1", features = ["v4", "serde"] }
  log = "0.4"
  env_logger = "0.11"
  anyhow = "1"
  thiserror = "1"
  ```
- **Tests Required**:
  - Verify `cargo check --manifest-path rust/server/Cargo.toml` succeeds

**1.2 Create error types**
- **File**: `rust/server/src/error.rs`
- **Purpose**: Centralized error handling
- **Changes**:
  ```rust
  // Define ConnectServerError enum
  // Implement Display, From<mongodb::error::Error>, From<bcrypt::BcryptError>
  // Implement actix_web::ResponseError for HTTP error responses
  ```
- **Tests Required**:
  - Unit test: Error conversion from MongoDB errors
  - Unit test: Error conversion to HTTP responses

**1.3 Create configuration module**
- **File**: `rust/server/src/config/mod.rs`
- **Purpose**: Load and parse server configuration
- **Changes**:
  ```rust
  // Define ServerConfig, WorldConfig structs
  // Implement load_from_file() using config crate
  // Parse servers.toml
  ```
- **File**: `rust/server/config/servers.toml`
- **Purpose**: Server definitions
- **Changes**: Create sample configuration with 1 server, 2 worlds
- **Tests Required**:
  - Unit test: Parse valid TOML configuration
  - Unit test: Handle missing configuration file
  - Integration test: Load sample servers.toml

#### Phase 2: Database Layer (Day 1-2)

**2.1 Define MongoDB models**
- **File**: `rust/server/src/db/models.rs`
- **Purpose**: Data structures for database entities
- **Changes**:
  ```rust
  // Account struct (matches MongoDB schema)
  // Character struct (matches MongoDB schema)
  // Derive Serialize, Deserialize
  // Add helper methods (e.g., Account::verify_password)
  ```
- **Tests Required**:
  - Unit test: Serialize/deserialize Account
  - Unit test: Verify password with bcrypt
  - Unit test: Password hash generation

**2.2 Implement repository pattern**
- **File**: `rust/server/src/db/repository.rs`
- **Purpose**: Database operations abstraction
- **Changes**:
  ```rust
  // AccountRepository trait + implementation
  //   - find_by_username()
  //   - create_account()
  //   - update_last_login()
  // CharacterRepository trait + implementation
  //   - find_by_account_id()
  //   - create_character()
  // MongoDbContext struct (holds MongoDB client)
  ```
- **Tests Required**:
  - Integration test: Connect to MongoDB (use testcontainers)
  - Integration test: Create and retrieve account
  - Integration test: Query characters by account_id
  - Integration test: Unique username constraint

**2.3 Database initialization**
- **File**: `rust/server/src/db/mod.rs`
- **Purpose**: MongoDB connection and index creation
- **Changes**:
  ```rust
  // init_database() function
  // Create indexes (username unique, account_id on characters)
  // Connection pool setup
  ```
- **Tests Required**:
  - Integration test: Index creation
  - Integration test: Connection pool reuse

#### Phase 3: Session Management (Day 2)

**3.1 Session manager implementation**
- **File**: `rust/server/src/session/manager.rs`
- **Purpose**: In-memory session storage
- **Changes**:
  ```rust
  // SessionManager struct with RwLock<HashMap>
  // SessionData struct
  // Methods:
  //   - create_session()
  //   - validate_session()
  //   - get_session()
  //   - invalidate_session()
  //   - check_duplicate_login() (returns old session to kick)
  // Background cleanup task (remove expired sessions)
  ```
- **Tests Required**:
  - Unit test: Create and retrieve session
  - Unit test: Session expiration
  - Unit test: Duplicate login detection (account level)
  - Unit test: Duplicate character login detection
  - Unit test: Cleanup task removes expired sessions
  - Concurrency test: Multiple concurrent session operations

**3.2 Session middleware**
- **File**: `rust/server/src/middleware/auth.rs`
- **Purpose**: Extract and validate session from cookie
- **Changes**:
  ```rust
  // AuthMiddleware (actix-web middleware)
  // Extract session cookie
  // Validate with SessionManager
  // Add session data to request extensions
  ```
- **Tests Required**:
  - Integration test: Middleware allows valid session
  - Integration test: Middleware rejects invalid session
  - Integration test: Middleware rejects missing cookie

#### Phase 4: Health Monitor (Day 2)

**4.1 Heartbeat monitor**
- **File**: `rust/server/src/monitor/health.rs`
- **Purpose**: Track game server health
- **Changes**:
  ```rust
  // HealthMonitor struct with RwLock<HashMap<WorldId, Heartbeat>>
  // Heartbeat struct (timestamp, player_count)
  // Methods:
  //   - record_heartbeat()
  //   - is_world_online()
  //   - get_world_status()
  //   - cleanup_stale_heartbeats() (background task)
  // Constants: HEARTBEAT_TIMEOUT = 30s
  ```
- **Tests Required**:
  - Unit test: Record heartbeat
  - Unit test: World marked offline after timeout
  - Unit test: World marked online after heartbeat
  - Unit test: Cleanup task removes stale entries

#### Phase 5: HTTP Handlers (Day 3)

**5.1 Authentication handlers**
- **File**: `rust/server/src/handlers/auth.rs`
- **Purpose**: Login and logout endpoints
- **Changes**:
  ```rust
  // POST /login handler
  //   - Extract username/password from JSON body
  //   - Query AccountRepository
  //   - Verify password with bcrypt
  //   - Check duplicate login (kick old session)
  //   - Create session
  //   - Set session cookie
  //   - Update last_login in DB
  // POST /logout handler
  //   - Extract session from middleware
  //   - Invalidate session
  //   - Clear cookie
  ```
- **Tests Required**:
  - Integration test: Login with valid credentials
  - Integration test: Login with invalid credentials
  - Integration test: Login kicks old session
  - Integration test: Logout invalidates session
  - Integration test: Session cookie is HttpOnly and SameSite

**5.2 Server listing handlers**
- **File**: `rust/server/src/handlers/servers.rs`
- **Purpose**: Server and world discovery
- **Changes**:
  ```rust
  // GET /servers handler
  //   - Load server config
  //   - Query HealthMonitor for world statuses
  //   - Aggregate world count per server
  //   - Return JSON response
  // GET /worlds handler
  //   - Load world config
  //   - Query HealthMonitor for each world
  //   - Filter to only online worlds
  //   - Return JSON with IP, port, player count
  ```
- **Tests Required**:
  - Integration test: GET /servers returns all servers
  - Integration test: GET /servers shows correct world_count
  - Integration test: GET /worlds returns only online worlds
  - Integration test: GET /worlds includes player count from heartbeat

**5.3 Character listing handler**
- **File**: `rust/server/src/handlers/characters.rs`
- **Purpose**: Return user's characters
- **Changes**:
  ```rust
  // GET /characters handler
  //   - Extract session from middleware (requires auth)
  //   - Query CharacterRepository by account_id
  //   - Return JSON response
  ```
- **Tests Required**:
  - Integration test: GET /characters with valid session
  - Integration test: GET /characters without session returns 401
  - Integration test: Returns correct characters for account

**5.4 Health check handlers**
- **File**: `rust/server/src/handlers/health.rs`
- **Purpose**: Heartbeat and health endpoints
- **Changes**:
  ```rust
  // POST /heartbeat handler
  //   - Extract world_id, player_count from JSON
  //   - Record heartbeat in HealthMonitor
  //   - Return success response
  // GET /health handler
  //   - Check MongoDB connection
  //   - Return 200 if healthy, 503 if unhealthy
  ```
- **Tests Required**:
  - Integration test: POST /heartbeat updates monitor
  - Integration test: GET /health returns 200 when DB connected
  - Integration test: GET /health returns 503 when DB disconnected

**5.5 Rate limiting middleware**
- **File**: `rust/server/src/middleware/rate_limit.rs`
- **Purpose**: Prevent brute force login attempts
- **Changes**:
  ```rust
  // RateLimitMiddleware (actix-web middleware)
  // Track requests per IP in RwLock<HashMap<IpAddr, Vec<Instant>>>
  // Limit: 10 requests per minute
  // Return 429 Too Many Requests if exceeded
  ```
- **Tests Required**:
  - Integration test: Allow 10 requests from same IP
  - Integration test: Block 11th request from same IP
  - Integration test: Allow requests after 1 minute

#### Phase 6: Application Assembly (Day 3)

**6.1 Main application**
- **File**: `rust/server/src/main.rs`
- **Purpose**: Initialize and start server
- **Changes**:
  ```rust
  // Load configuration
  // Initialize MongoDB connection
  // Create SessionManager (Arc<SessionManager>)
  // Create HealthMonitor (Arc<HealthMonitor>)
  // Spawn background tasks:
  //   - Session cleanup
  //   - Heartbeat timeout monitor
  // Configure actix-web App:
  //   - Add middleware (logging, rate limit, auth)
  //   - Register routes
  //   - Share state (SessionManager, HealthMonitor, DB)
  // Bind to 0.0.0.0:8080
  // Start server
  ```
- **Tests Required**:
  - Integration test: Server starts and responds to /health
  - Integration test: Full flow (login → get characters → logout)

#### Phase 7: Testing & Documentation (Day 4-5)

**7.1 End-to-End Tests**
- **File**: `rust/server/tests/e2e_tests.rs`
- **Purpose**: Validate complete workflows
- **Test Cases**:
  - Full user journey: login → list servers → list worlds → list characters → logout
  - Duplicate login scenario: user1 logs in twice, first session kicked
  - Heartbeat scenario: world online → heartbeat stops → world offline
  - Rate limiting: 11 login attempts from same IP

**7.2 Load Testing**
- **File**: `rust/server/tests/load_test.rs` (using `criterion` or external tool)
- **Purpose**: Validate 1000 concurrent user requirement
- **Test Cases**:
  - 1000 concurrent logins
  - 1000 concurrent /characters requests
  - Measure p95 latency < 200ms for login

**7.3 Documentation**
- **File**: `rust/server/README.md`
- **Purpose**: Setup and usage instructions
- **Content**:
  - Prerequisites (MongoDB, Rust)
  - Configuration guide (servers.toml)
  - Running the server
  - API endpoint documentation
  - Testing instructions
  - Migration path to Redis (future enhancement)

**7.4 Database Seeding Script**
- **File**: `rust/server/src/bin/seed_db.rs`
- **Purpose**: Create sample accounts and characters
- **Changes**:
  ```rust
  // Create 5 sample accounts with bcrypt passwords
  // Create 10 sample characters across accounts
  // Useful for testing and development
  ```
- **Tests Required**:
  - Manual test: Run seed script and verify data in MongoDB

## 🧪 Testing Strategy

### Test Coverage Requirements

**Unit Tests (minimum 80% coverage)**:
- [ ] Configuration loading and parsing
- [ ] Password hashing and verification
- [ ] Session creation, validation, expiration
- [ ] Duplicate login detection
- [ ] Heartbeat timeout logic
- [ ] Error type conversions
- [ ] Data model serialization

**Integration Tests**:
- [ ] MongoDB CRUD operations
- [ ] All HTTP endpoints (success and error cases)
- [ ] Session middleware flow
- [ ] Rate limiting middleware
- [ ] Background cleanup tasks
- [ ] Full authentication flow

**End-to-End Tests**:
- [ ] Complete user journey (login → list data → logout)
- [ ] Duplicate login kicks old session
- [ ] Heartbeat affects world status
- [ ] Unauthenticated requests blocked

**Load Tests**:
- [ ] 1000 concurrent login requests
- [ ] p95 latency < 200ms for login endpoint
- [ ] Session manager handles concurrent access

### Test Cases Table

| Test ID | Type | Description | Expected Result |
|---------|------|-------------|-----------------|
| TC001 | Unit | Hash password with bcrypt | Hashed password stored, original not retrievable |
| TC002 | Unit | Verify correct password | Returns true |
| TC003 | Unit | Verify incorrect password | Returns false |
| TC004 | Unit | Create session | Session ID generated, data stored |
| TC005 | Unit | Validate session with valid ID | Returns session data |
| TC006 | Unit | Validate session with expired ID | Returns error |
| TC007 | Unit | Detect duplicate account login | Returns old session ID to invalidate |
| TC008 | Unit | Heartbeat marks world online | is_world_online() returns true |
| TC009 | Unit | Heartbeat timeout marks offline | After 30s, is_world_online() returns false |
| TC010 | Integration | POST /login valid creds | 200 + session cookie + account_id |
| TC011 | Integration | POST /login invalid creds | 401 + error message |
| TC012 | Integration | POST /login duplicate | 409 + old session kicked |
| TC013 | Integration | GET /characters authenticated | 200 + character list |
| TC014 | Integration | GET /characters unauthenticated | 401 + error |
| TC015 | Integration | GET /servers | 200 + server list with status |
| TC016 | Integration | GET /worlds | 200 + online worlds only |
| TC017 | Integration | POST /heartbeat | 200 + updates health monitor |
| TC018 | Integration | GET /health (DB connected) | 200 |
| TC019 | Integration | Rate limit exceeded | 429 after 10 requests |
| TC020 | E2E | Full user flow | Login → list chars → logout succeeds |
| TC021 | Load | 1000 concurrent logins | All succeed, p95 < 200ms |

## 🚀 Deployment & Operations

### Environment Variables

```bash
# .env file
MONGODB_URI=mongodb://localhost:27017
DATABASE_NAME=mu_online
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
SESSION_EXPIRY_HOURS=24
BCRYPT_COST=12
LOG_LEVEL=info
```

### Running the Server

```bash
# Development
cd rust/server
cargo run

# Production (release build)
cargo build --release --manifest-path rust/server/Cargo.toml
./rust/target/release/server
```

### MongoDB Setup

```bash
# Start MongoDB (Docker)
docker run -d -p 27017:27017 --name mongodb mongo:latest

# Create indexes (automatic on first run)
# Or manually:
mongo mu_online --eval '
  db.accounts.createIndex({username: 1}, {unique: true});
  db.characters.createIndex({account_id: 1});
  db.characters.createIndex({name: 1}, {unique: true});
'
```

### Monitoring & Observability

**Metrics to track**:
- Active session count (in-memory)
- Login success/failure rate
- Average login latency
- MongoDB connection pool usage
- Heartbeat success rate per world

**Logs to implement**:
- Login attempts (username, IP, success/failure)
- Session creation/invalidation
- Heartbeat received (world_id, player_count)
- World status changes (online ↔ offline)
- Database errors
- Rate limit violations

**Alerts to configure** (future, when monitoring added):
- Login failure rate > 10% over 5 minutes
- MongoDB connection failures
- No heartbeat from any world for > 1 minute
- Session count > 800 (approaching 1000 limit)

## ⚠️ Risk Assessment

| Risk | Probability | Impact | Mitigation Strategy |
|------|------------|--------|-------------------|
| In-memory sessions lost on restart | High | Medium | Document Redis migration path; acceptable for MVP |
| MongoDB connection failures | Medium | High | Implement connection retry logic with exponential backoff |
| Session manager lock contention at scale | Medium | Medium | Use DashMap instead of RwLock<HashMap> if contention detected |
| Bcrypt slows down login under load | Low | Medium | Use async bcrypt (spawn_blocking), consider caching recent verifications |
| Heartbeat spam from malicious game servers | Low | Low | Add authentication to /heartbeat endpoint (shared secret) |
| Rate limiting bypassed via IP spoofing | Medium | Low | Move to reverse proxy rate limiting (nginx, Cloudflare) in production |
| Password timing attacks | Low | Medium | Bcrypt is constant-time; ensure no early returns in validation logic |

## 📅 Implementation Timeline

### Phase 1: Foundation (Day 1)
- [x] Research MongoDB Rust driver
- [ ] Set up project structure and dependencies
- [ ] Create error types and configuration module
- [ ] Implement server configuration loading from TOML
- [ ] Write configuration loading tests

### Phase 2: Database Layer (Day 1-2)
- [ ] Define MongoDB data models (Account, Character)
- [ ] Implement repository pattern for database operations
- [ ] Create database initialization and indexing
- [ ] Write integration tests with testcontainers
- [ ] Seed database script for testing

### Phase 3: Core Services (Day 2)
- [ ] Implement SessionManager with in-memory storage
- [ ] Implement duplicate login detection
- [ ] Implement HealthMonitor for heartbeat tracking
- [ ] Create background cleanup tasks
- [ ] Write unit tests for session and health logic

### Phase 4: HTTP Layer (Day 3)
- [ ] Implement authentication handlers (login, logout)
- [ ] Implement server/world listing handlers
- [ ] Implement character listing handler
- [ ] Implement heartbeat and health check handlers
- [ ] Create session validation middleware
- [ ] Create rate limiting middleware
- [ ] Write integration tests for all endpoints

### Phase 5: Integration & Testing (Day 4)
- [ ] Assemble application in main.rs
- [ ] Write end-to-end tests
- [ ] Perform load testing (1000 concurrent users)
- [ ] Fix any performance bottlenecks
- [ ] Code review and refactoring

### Phase 6: Documentation & Polish (Day 5)
- [ ] Write README with setup instructions
- [ ] Document API endpoints
- [ ] Add inline code documentation
- [ ] Create example curl commands for testing
- [ ] Document migration path to Redis
- [ ] Final clippy and fmt pass

## ✅ Definition of Done

- [ ] All acceptance criteria met for functional requirements
- [ ] Code passes `cargo clippy` with no warnings
- [ ] Code formatted with `cargo fmt`
- [ ] Unit test coverage ≥ 80%
- [ ] All integration tests passing
- [ ] End-to-end test suite passing
- [ ] Load test validates 1000 concurrent users
- [ ] API endpoints match specification
- [ ] Session management prevents duplicate logins
- [ ] Heartbeat system correctly tracks world status
- [ ] MongoDB connection with proper error handling
- [ ] Bcrypt password hashing implemented
- [ ] Rate limiting protects login endpoint
- [ ] Structured logging throughout application
- [ ] README documentation complete
- [ ] Configuration file example provided
- [ ] Database seeding script functional
- [ ] No panics or unwraps in production code paths
- [ ] Graceful shutdown handling

## 📚 Documentation Updates Required

- [ ] `rust/server/README.md` - Setup, configuration, API reference
- [ ] `rust/server/config/servers.toml.example` - Sample configuration
- [ ] `.env.example` - Environment variable template
- [ ] `docs/architecture/connect-server.md` - Architecture overview (optional)
- [ ] Update root `README.md` to mention connect server

## 💬 Open Questions & Blockers

### Resolved Questions
1. ✅ **Database choice**: MongoDB confirmed
2. ✅ **Authentication method**: Username/password with bcrypt
3. ✅ **Session storage**: In-memory for MVP
4. ✅ **Duplicate login handling**: Kick old session
5. ✅ **Web framework**: actix-web
6. ✅ **Server discovery**: Static configuration file
7. ✅ **Health check mechanism**: Heartbeat with 30s timeout

### Remaining Questions
1. **Character selection flow**: Should `GET /characters` be called before connecting to a world server? Or does the client select a character and the world server validates with connect server?
   - **Impact**: May need a "select character" endpoint that locks the character to the session

2. **World server authentication**: How should world servers authenticate when sending heartbeats?
   - **Recommendation**: Shared secret in configuration, sent as `Authorization` header

3. **Session transfer to game server**: When a player connects to a world server, how does the world server validate the session?
   - **Recommendation**: Client sends session token, world server calls `GET /validate-session` on connect server

## 🔄 Alternative Approaches Considered

| Approach | Pros | Cons | Why not chosen |
|----------|------|------|----------------|
| **Redis for sessions** | Persists across restarts, supports horizontal scaling | Additional infrastructure dependency | In-memory acceptable for MVP; migration path documented |
| **PostgreSQL instead of MongoDB** | ACID guarantees, relational integrity | More rigid schema, slower for document-heavy workloads | User specified MongoDB |
| **JWT tokens instead of sessions** | Stateless, easier to scale | Cannot revoke until expiration, larger payload | User specified traditional session cookies |
| **gRPC instead of REST** | Better performance, type safety | More complex client integration, less HTTP-friendly | User specified REST+JSON |
| **Diesel ORM** | Type-safe SQL, compile-time checks | PostgreSQL/MySQL only, not MongoDB | MongoDB chosen as database |
| **Rocket instead of actix-web** | Simpler API, better ergonomics | Less mature ecosystem, slower performance | User specified actix-web |

## 🔮 Future Enhancements

1. **Redis Migration**: Move session storage to Redis for persistence and horizontal scaling
2. **Character Selection Endpoint**: `POST /select-character` to lock character to session
3. **World Server API**: Dedicated endpoints for world servers to validate sessions
4. **Admin API**: Endpoints for managing accounts, characters, servers
5. **Metrics & Observability**: Prometheus metrics, distributed tracing
6. **Hot Configuration Reload**: Reload `servers.toml` without restart
7. **IP Geolocation**: Route users to nearest server
8. **Queue System**: Handle server capacity limits with player queues
9. **Two-Factor Authentication**: Optional 2FA for accounts
10. **OAuth2 Support**: Allow login via Discord, Google, etc.

---

## Quick Start Checklist

When beginning implementation, follow this order:

1. [ ] Set up MongoDB (Docker or local install)
2. [ ] Update `rust/server/Cargo.toml` with dependencies
3. [ ] Create project structure (src/config, src/db, src/handlers, etc.)
4. [ ] Implement configuration loading and test with sample `servers.toml`
5. [ ] Implement database models and repository
6. [ ] Test MongoDB connection and CRUD operations
7. [ ] Implement SessionManager
8. [ ] Implement HealthMonitor
9. [ ] Implement HTTP handlers one by one
10. [ ] Wire everything together in `main.rs`
11. [ ] Run integration tests
12. [ ] Perform load testing
13. [ ] Document and deploy

---

**Plan created**: 2025-10-22
**Last updated**: 2025-10-22
**Status**: Ready for implementation
