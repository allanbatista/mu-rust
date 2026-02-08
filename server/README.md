# MU Online Connect Server

The Connect Server is a microservice responsible for user authentication, session management, and game server discovery in the MU Online ecosystem.

## Features

- **User Authentication**: Username/password authentication with bcrypt hashing
- **Session Management**: In-memory session storage with automatic cleanup
- **Duplicate Login Prevention**: Automatically kicks old sessions when users log in from a new location
- **Server Discovery**: Lists available game servers
- **World Discovery**: Lists available world instances with IP/port for client connection
- **Character Management**: Retrieve user's characters
- **Health Monitoring**: Heartbeat-based system for tracking world server health
- **Rate Limiting**: Protection against brute force login attempts (10 req/min per IP)
- **MU Core Runtime**: world/entry/map runtime with one `MapServer` per map instance
- **QUIC Gateway**: binary protocol v2 ingress (stream + datagram)
- **Buffered Persistence**: coalesced flush for non-critical state + immediate critical events

## Architecture

The server is built with:
- **actix-web**: HTTP REST API framework
- **MongoDB**: Database for accounts and characters
- **In-memory sessions**: DashMap for concurrent session storage
- **Background tasks**: Automatic cleanup of expired sessions and stale heartbeats
- **MU Core**:
  - `WorldDirectory` for routing and occupancy
  - `MapServer` per map instance (character-priority tick loop)
  - `MessageHub` for chat/event fanout
  - `PersistenceWorker` for write buffering
  - `QUIC Gateway` for protocol v2 transport

### Protocol Roadmap

The workspace protocol uses a QUIC-ready typed protocol (`protocol v2`) end-to-end.

- Current protocol spec: `docs/architecture/protocol-v2-quic.md`
- Migration phases and rollout: `docs/architecture/protocol-migration-roadmap.md`

## API Endpoints

### Public Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/login` | Authenticate user and create session |
| GET | `/servers` | List available game servers |
| GET | `/worlds` | List online world instances |
| POST | `/heartbeat` | Game server health check |
| GET | `/health` | Connect server health status |
| GET | `/runtime/worlds` | Runtime topology snapshot (world/entry/map) |
| GET | `/runtime/maps` | Runtime map loop metrics |
| GET | `/runtime/persistence` | Buffered persistence metrics |
| GET | `/runtime/stats` | Runtime high-level stats |

### Protected Endpoints (Require Authentication)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/logout` | Invalidate current session |
| GET | `/characters` | List user's characters |

## Prerequisites

- Rust 1.70+ (edition 2021)
- MongoDB 3.6+
- Environment variables (optional, see Configuration)

## Configuration

### Environment Variables

Create a `.env` file based on `.env.example`:

```bash
cp .env.example .env
```

Configure the following variables:

```bash
# MongoDB connection (with authentication)
# Make sure this matches the credentials in rust/docker/.env
MONGODB_URI=mongodb://admin:admin123@localhost:27017/mu?authSource=admin
DATABASE_NAME=mu

# Server configuration
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# Session settings
SESSION_EXPIRY_HOURS=24

# Runtime/QUIC
ENABLE_MU_CORE=true
ENABLE_QUIC_GATEWAY=true
RUNTIME_CONFIG_PATH=server/config/runtime.toml
QUIC_CERT_PATH=server/config/certs/server.crt   # optional
QUIC_KEY_PATH=server/config/certs/server.key    # optional

# Logging
RUST_LOG=info
```

**Important**: The `MONGODB_URI` must include authentication credentials when using the Docker setup from `rust/docker/`. The default credentials are `admin:admin123`, but you should change them in production.
If `QUIC_CERT_PATH`/`QUIC_KEY_PATH` are not set, the server generates a self-signed certificate on startup.

### Server Configuration File

Edit `config/servers.toml` to define your game servers and worlds:

```toml
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

## Running the Server

### Development Mode

```bash
# 1. Make sure you have a .env file (copy from .env.example)
cd rust/server
cp .env.example .env  # Edit if needed

# 2. Make sure MongoDB is running (see "With Docker" section below)

# 3. Run the server from the rust/ directory (important!)
cd ..  # Go back to rust/ directory
cargo run --manifest-path server/Cargo.toml
```

**Important Notes**:
- The server must be run from the `rust/` directory (not `rust/server/`) because it needs to find `server/config/servers.toml`
- The server automatically loads the `.env` file from `server/.env` using the `dotenvy` crate
- You don't need to export environment variables manually

### Production Mode

```bash
# Build release binary
cargo build --release --manifest-path rust/server/Cargo.toml

# Run
./rust/target/release/server
```

### With Docker (MongoDB)

The recommended way to run MongoDB for development is using the Docker Compose setup:

```bash
# 1. Start MongoDB and Mongo Express
cd ../docker
cp .env.example .env  # Edit if needed
docker-compose up -d

# 2. Configure the server
cd ../server
cp .env.example .env  # Make sure MONGODB_URI matches docker/.env credentials

# 3. Run the server
cargo run
```

MongoDB will be available at:
- **MongoDB**: `localhost:27017`
- **Mongo Express (Web UI)**: `http://localhost:8081`

See `rust/docker/README.md` for detailed MongoDB setup instructions.

## Testing

Run unit tests:

```bash
cargo test --manifest-path rust/server/Cargo.toml
```

Run specific tests:

```bash
# Test session management
cargo test --manifest-path rust/server/Cargo.toml session

# Test configuration
cargo test --manifest-path rust/server/Cargo.toml config
```

### Protocol + Login Simulation

Use the simulator client to validate concrete flows:
- HTTP login (`POST /login`)
- authenticated request (`GET /characters`)
- QUIC protocol v2 (`Hello` -> `HelloAck`, `KeepAlive` -> `Pong`)

Command:

```bash
./server/scripts/sim_client.sh \
  --http-base http://127.0.0.1:8080 \
  --username testuser \
  --password testpass \
  --quic-addr 127.0.0.1:6000 \
  --quic-server-name localhost \
  --quic-ca-cert server/config/certs/server.crt
```

If you only want login/session validation:

```bash
./server/scripts/sim_client.sh \
  --http-base http://127.0.0.1:8080 \
  --username testuser \
  --password testpass \
  --skip-quic
```

## API Examples

### Login

```bash
curl -X POST http://localhost:8080/login \
  -H "Content-Type: application/json" \
  -d '{"username": "player1", "password": "secret123"}' \
  -c cookies.txt
```

Response:
```json
{
  "success": true,
  "account_id": "507f1f77bcf86cd799439011",
  "message": "Login successful"
}
```

### List Servers

```bash
curl http://localhost:8080/servers
```

Response:
```json
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

### List Worlds

```bash
curl http://localhost:8080/worlds
```

Response:
```json
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

### List Characters (Authenticated)

```bash
curl http://localhost:8080/characters \
  -b cookies.txt
```

Response:
```json
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
```

### Logout

```bash
curl -X POST http://localhost:8080/logout \
  -b cookies.txt
```

### Heartbeat (Game Server)

```bash
curl -X POST http://localhost:8080/heartbeat \
  -H "Content-Type: application/json" \
  -d '{
    "world_id": "world-1-lorencia",
    "current_players": 42,
    "timestamp": 1634567890
  }'
```

## Database Setup

The server automatically creates the necessary MongoDB indexes on startup:

- `accounts.username` (unique)
- `characters.account_id`
- `characters.name` (unique)

### Manual Database Initialization

```bash
mongo mu_online --eval '
  db.accounts.createIndex({username: 1}, {unique: true});
  db.characters.createIndex({account_id: 1});
  db.characters.createIndex({name: 1}, {unique: true});
'
```

### Sample Data

Create a test account:

```bash
mongo mu_online --eval '
  db.accounts.insertOne({
    username: "testuser",
    password_hash: "$2b$12$...",
    created_at: new Date(),
    last_login: new Date()
  });
'
```

## Project Structure

```
server/
├── src/
│   ├── main.rs              # Application entry point
│   ├── config/              # Configuration loading
│   │   └── mod.rs
│   ├── db/                  # Database layer
│   │   ├── mod.rs
│   │   ├── models.rs        # Account, Character models
│   │   └── repository.rs    # MongoDB operations
│   ├── handlers/            # HTTP endpoint handlers
│   │   ├── mod.rs
│   │   ├── auth.rs          # Login, logout
│   │   ├── servers.rs       # Server/world listing
│   │   ├── characters.rs    # Character management
│   │   └── health.rs        # Heartbeat, health check
│   ├── middleware/          # HTTP middleware
│   │   ├── mod.rs
│   │   ├── auth.rs          # Session validation
│   │   └── rate_limit.rs    # Login rate limiting
│   ├── session/             # Session management
│   │   ├── mod.rs
│   │   └── manager.rs       # In-memory session store
│   ├── monitor/             # Health monitoring
│   │   ├── mod.rs
│   │   └── health.rs        # Heartbeat tracking
│   └── error.rs             # Error types
├── config/
│   └── servers.toml         # Server configuration
├── Cargo.toml
└── README.md
```

## Background Tasks

The server runs three background cleanup tasks:

1. **Session Cleanup** (every 60s): Removes expired sessions
2. **Heartbeat Monitor** (every 60s): Marks worlds offline after 30s timeout
3. **Rate Limiter Cleanup** (every 5min): Cleans old rate limit entries

## Security Features

- **Bcrypt Password Hashing**: Cost factor 12
- **Secure Session Cookies**: HttpOnly, SameSite=Strict
- **Rate Limiting**: 10 login requests per minute per IP
- **No Password Logging**: Passwords never appear in logs
- **Session Validation**: All protected endpoints validate session before processing

## Performance

- **Target**: 1000 concurrent users
- **Session Lookup**: <10ms (in-memory DashMap)
- **Login Latency**: <200ms p95 (with bcrypt verification)

## Future Enhancements

- [ ] Redis session storage for horizontal scaling
- [ ] Character selection endpoint
- [ ] World server session validation API
- [ ] Admin API for account/character management
- [ ] Prometheus metrics integration
- [ ] Hot reload of server configuration
- [ ] Two-factor authentication
- [ ] OAuth2 support

## Troubleshooting

### MongoDB Connection Failed

```bash
# Check if MongoDB is running
mongosh --eval "db.adminCommand('ping')"

# Start MongoDB without authentication (development)
docker run -d -p 27017:27017 --name mongodb mongo:latest

# Start MongoDB with authentication (production)
docker run -d -p 27017:27017 \
  -e MONGO_INITDB_ROOT_USERNAME=admin \
  -e MONGO_INITDB_ROOT_PASSWORD=password \
  --name mongodb mongo:latest

# Then set connection string with credentials
export MONGODB_URI=mongodb://admin:password@localhost:27017
```

### MongoDB Authentication Error

If you see `"Command createIndexes requires authentication"`:

```bash
# Option 1: Use MongoDB without authentication (development only)
docker stop mongodb
docker rm mongodb
docker run -d -p 27017:27017 --name mongodb mongo:latest

# Option 2: Set credentials in environment
export MONGODB_URI=mongodb://username:password@localhost:27017
```

### Configuration File Not Found

If you see `"Failed to read config file: No such file or directory"`:

```bash
# Option 1: Run from the rust/ directory
cd rust/
cargo run --manifest-path server/Cargo.toml

# Option 2: Set CONFIG_PATH environment variable
export CONFIG_PATH=server/config/servers.toml
cargo run --manifest-path server/Cargo.toml

# Option 3: Use absolute path
export CONFIG_PATH=/absolute/path/to/servers.toml
```

### Port Already in Use

```bash
# Change port in environment
export SERVER_PORT=8081

# Or kill existing process
lsof -ti:8080 | xargs kill -9
```

### Tests Failing

```bash
# Ensure MongoDB is accessible for integration tests
docker run -d -p 27017:27017 --name mongodb-test mongo:latest

# Run tests
cargo test
```

## Contributing

When contributing to the Connect Server:

1. Run `cargo fmt` before committing
2. Run `cargo clippy --all-targets -- -D warnings` to check for lint issues
3. Ensure all tests pass with `cargo test`
4. Update this README if adding new features or endpoints

## License

See repository root for license information.
