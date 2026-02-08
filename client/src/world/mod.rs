use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::{AmbientLight, DirectionalLight, DirectionalLightBundle};
use bevy::prelude::*;
use bevy::render::camera::{ClearColorConfig, PerspectiveProjection, Projection};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldId {
    Loading,
    Login,
}

#[derive(Event)]
pub struct WorldRequest(pub WorldId);

#[derive(Event)]
pub struct WorldReady(pub WorldId);

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
            .add_event::<WorldRequest>()
            .add_event::<WorldReady>()
            .insert_resource(AmbientLight {
                color: Color::WHITE,
                brightness: 0.3,
            })
            .add_systems(Update, process_world_requests)
            .add_systems(Startup, setup_world_camera);
    }
}

fn process_world_requests(
    mut commands: Commands,
    mut current_world: ResMut<CurrentWorld>,
    mut requests: EventReader<WorldRequest>,
    mut ready_writer: EventWriter<WorldReady>,
    roots: Query<Entity, With<WorldRoot>>,
) {
    for WorldRequest(requested) in requests.read() {
        // Despawn existing world entities
        for entity in &roots {
            commands.entity(entity).despawn();
        }

        spawn_world(&mut commands, *requested);
        current_world.0 = Some(*requested);
        ready_writer.send(WorldReady(*requested));
    }
}

fn spawn_world(commands: &mut Commands, world_id: WorldId) {
    match world_id {
        WorldId::Loading => {
            commands.spawn((WorldRoot, SpatialBundle::default()));
        }
        WorldId::Login => {
            // Placeholder; actual assets/camera setup will be added later
            commands.spawn((WorldRoot, SpatialBundle::default()));
        }
    }
}

fn setup_world_camera(mut commands: Commands) {
    // 3D Camera for world rendering
    commands.spawn((
        WorldCamera,
        Camera3dBundle {
            camera: Camera {
                order: 0, // Render 3D world first
                clear_color: ClearColorConfig::Custom(Color::srgb(0.1, 0.1, 0.15)),
                ..Default::default()
            },
            tonemapping: Tonemapping::None,
            projection: Projection::Perspective(PerspectiveProjection {
                near: 10.0,
                far: 50_000.0,
                ..default()
            }),
            transform: Transform::from_xyz(24_920.0, 520.0, 2_500.0)
                .looking_at(Vec3::new(24_056.0, 170.0, 2_500.0), Vec3::Y),
            ..Default::default()
        },
    ));

    // 2D Camera for UI overlay
    commands.spawn(Camera2dBundle {
        camera: Camera {
            order: 1,                            // Render UI on top
            clear_color: ClearColorConfig::None, // Don't clear, draw over 3D
            ..Default::default()
        },
        tonemapping: Tonemapping::None,
        ..Default::default()
    });

    // Directional light (sun)
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: false,
            ..Default::default()
        },
        transform: Transform::from_xyz(20_000.0, 6_000.0, 2_000.0)
            .looking_at(Vec3::new(22_000.0, 170.0, 2_500.0), Vec3::Y),
        ..Default::default()
    });
}
