use bevy::camera::{ClearColorConfig, PerspectiveProjection, Projection};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::GlobalAmbientLight;
use bevy::prelude::*;
use common::WorldMap;

/// Represents the current world/map being displayed in the client
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldId {
    /// Loading screen
    Loading,
    /// Login/Character selection scene (uses a specific map)
    Login(WorldMap),
    /// Gameplay world
    Game(WorldMap),
}

#[derive(Message)]
pub struct WorldRequest(pub WorldId);

#[derive(Message)]
pub struct WorldReady;

#[derive(Component)]
struct WorldRoot;
#[derive(Component)]
struct WorldCamera;

#[derive(Resource, Default)]
pub struct CurrentWorld(pub Option<WorldId>);

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentWorld>()
            .add_message::<WorldRequest>()
            .add_message::<WorldReady>()
            .insert_resource(GlobalAmbientLight {
                color: Color::WHITE,
                brightness: 0.3,
                affects_lightmapped_meshes: true,
            })
            .add_systems(Update, process_world_requests)
            .add_systems(Startup, setup_world_camera);
    }
}

fn process_world_requests(
    mut commands: Commands,
    mut current_world: ResMut<CurrentWorld>,
    mut requests: MessageReader<WorldRequest>,
    mut ready_writer: MessageWriter<WorldReady>,
    roots: Query<Entity, With<WorldRoot>>,
) {
    for WorldRequest(requested) in requests.read() {
        // Despawn existing world entities
        for entity in &roots {
            commands.entity(entity).try_despawn();
        }

        spawn_world(&mut commands, *requested);
        current_world.0 = Some(*requested);
        ready_writer.write(WorldReady);
    }
}

fn spawn_world(commands: &mut Commands, world_id: WorldId) {
    match world_id {
        WorldId::Loading => {
            info!("Spawning loading world");
            commands.spawn(WorldRoot);
        }
        WorldId::Login(map) => {
            info!("Spawning login world: {} (ID: {})", map.name(), map as u8);
            commands.spawn(WorldRoot);
        }
        WorldId::Game(map) => {
            info!("Spawning game world: {} (ID: {})", map.name(), map as u8);
            commands.spawn(WorldRoot);
        }
    }
}

fn setup_world_camera(mut commands: Commands) {
    // 3D Camera for world rendering
    commands.spawn((
        WorldCamera,
        Camera3d::default(),
        Camera {
            order: 0, // Render 3D world first
            clear_color: ClearColorConfig::Custom(Color::srgb(0.1, 0.1, 0.15)),
            ..Default::default()
        },
        Tonemapping::None,
        Projection::Perspective(PerspectiveProjection {
            near: 10.0,
            far: 50_000.0,
            ..default()
        }),
        Transform::from_xyz(24_920.0, 520.0, 2_500.0)
            .looking_at(Vec3::new(24_056.0, 170.0, 2_500.0), Vec3::Y),
    ));
}
