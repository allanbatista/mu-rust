use server::config::ServerConfig;

#[test]
fn test_load_server_config() {
    let config =
        ServerConfig::load_from_file("server/config/servers.toml").expect("Failed to load config");

    assert_eq!(config.servers.len(), 2);
}

#[test]
fn test_config_server_structure() {
    let config =
        ServerConfig::load_from_file("server/config/servers.toml").expect("Failed to load config");

    let server = &config.servers[0];
    assert_eq!(server.id, "server-1");
    assert_eq!(server.name, "Alpha Server");
    assert!(!server.description.is_empty());
    assert!(!server.worlds.is_empty());
}

#[test]
fn test_config_world_structure() {
    let config =
        ServerConfig::load_from_file("server/config/servers.toml").expect("Failed to load config");

    let world = &config.servers[0].worlds[0];
    assert_eq!(world.id, "world-1-lorencia");
    assert_eq!(world.name, "Lorencia");
    assert!(!world.ip.is_empty());
    assert!(world.port > 0);
    assert!(world.max_players > 0);
}

#[test]
fn test_invalid_config_path() {
    let result = ServerConfig::load_from_file("nonexistent/config.toml");
    assert!(result.is_err());
}

#[test]
fn test_all_worlds_have_unique_ids() {
    let config =
        ServerConfig::load_from_file("server/config/servers.toml").expect("Failed to load config");

    let mut world_ids = std::collections::HashSet::new();

    for server in &config.servers {
        for world in &server.worlds {
            assert!(
                world_ids.insert(&world.id),
                "Duplicate world ID found: {}",
                world.id
            );
        }
    }
}

#[test]
fn test_all_servers_have_unique_ids() {
    let config =
        ServerConfig::load_from_file("server/config/servers.toml").expect("Failed to load config");

    let mut server_ids = std::collections::HashSet::new();

    for server in &config.servers {
        assert!(
            server_ids.insert(&server.id),
            "Duplicate server ID found: {}",
            server.id
        );
    }
}
