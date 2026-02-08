# Using WorldMap Enum in Client

The `common` crate provides a `WorldMap` enum that represents all 82 maps/worlds in MU Online. This is now integrated into the client for specifying which world/login scene to use.

## Architecture

```
common crate
└── WorldMap enum (82 variants)
    ├── Lorencia, Dungeon, Devias, ...
    ├── LoginScene (ID: 55)
    ├── NewLoginScene1 (ID: 73)
    ├── NewLoginScene2 (ID: 77)
    └── Helper methods: is_login_scene(), is_pvp_area(), name(), etc.

client crate
└── WorldId enum
    ├── Loading
    ├── Login(WorldMap)  ← Specifies which login world to use
    └── Game(WorldMap)   ← Future: Specifies which game world to use
```

## Using in Login Scene

The login scene automatically loads the world specified by the `MU_LOGIN_WORLD` environment variable.

### Example 1: Use default login world (ID: 55)

```bash
cargo run -p client
# Uses WorldMap::LoginScene
```

### Example 2: Use new login scene v1 (ID: 73)

```bash
MU_LOGIN_WORLD=73 cargo run -p client
# Uses WorldMap::NewLoginScene1
```

### Example 3: Use new login scene v2 (ID: 77)

```bash
MU_LOGIN_WORLD=77 cargo run -p client
# Uses WorldMap::NewLoginScene2
```

## Code Example

```rust
use common::WorldMap;
use crate::world::{WorldId, WorldRequest};

// Create a login world request with a specific map
let world = WorldMap::LoginScene;
world_requests.send(WorldRequest(WorldId::Login(world)));

// Log the world name and ID
println!("Loaded world: {} (ID: {})", world.name(), world as u8);

// Check if it's a login scene
if world.is_login_scene() {
    println!("This is a login/character selection scene");
}
```

## Valid Login Worlds

| ID | WorldMap Variant | Description |
|----|-----------------|-------------|
| 55 | `LoginScene` | Original login scene background |
| 73 | `NewLoginScene1` | New login scene (version 1) |
| 74 | `NewCharacterScene1` | New character selection (version 1) |
| 77 | `NewLoginScene2` | New login scene (version 2) |
| 78 | `NewCharacterScene2` | New character selection (version 2) |

## All Available Worlds

The `WorldMap` enum includes 82 variants organized by category:

- **Main Worlds** (11): Lorencia, Dungeon, Devias, Noria, Lost Tower, Unknown, Stadium, Atlantis, Tarkan, Devil Square, Heaven
- **Blood Castle** (7 levels): BloodCastle1 through BloodCastle7
- **Chaos Castle** (6 levels): ChaosCastle1 through ChaosCastle6
- **Hellas** (3 levels): Hellas1, Hellas2, Hellas3
- **Event Maps**: CryWolf1st, CryWolf2nd, Kanturu1st-3rd, etc.
- **PvP Areas**: Stadium, PKField, DuelArena
- **Special**: BattleCastle, HuntingGround, SantaTown, etc.

For the complete list, see `rust/common/src/lib.rs`.

## Adding Support for Game Worlds

To switch to a game world after login:

```rust
// Example: Switch to Lorencia world
world_requests.send(WorldRequest(WorldId::Game(WorldMap::Lorencia)));

// With ID conversion
if let Some(map) = WorldMap::from_id(0) {  // 0 is Lorencia
    world_requests.send(WorldRequest(WorldId::Game(map)));
}
```

## Helper Methods

All `WorldMap` variants have helpful methods:

```rust
let map = WorldMap::LoginScene;

// Get readable name
println!("{}", map.name());  // Output: "Login Scene"

// Get numeric ID
println!("{}", map as u8);   // Output: 55

// Check world type
if map.is_login_scene() { ... }
if map.is_pvp_area() { ... }
if map.is_event_dungeon() { ... }

// Create from ID
if let Some(map) = WorldMap::from_id(55) {
    println!("Found map: {}", map.name());
}

// Display trait
println!("{}", map);  // Output: "Login Scene"
```

## Testing

Run tests for the `common` crate:

```bash
cargo test -p common
```

Expected output:
```
running 4 tests
test tests::test_from_id ... ok
test tests::test_is_login_scene ... ok
test tests::test_world_map_creation ... ok
test tests::test_is_pvp_area ... ok
```
