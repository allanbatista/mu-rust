use bevy::prelude::*;
use bevy::state::app::AppExtStates;

use client::AppState;
use client::gameplay::controllers::scene_controller::{SceneControllerPlugin, transition_to};
use client::gameplay::scenes::gameplay::GameplayScene;
use client::gameplay::scenes::login::LoginScene;
use client::scene_runtime::components::RuntimeSceneEntity;
use client::scene_runtime::state::RuntimeSceneAssets;
use client::world::WorldRequest;

fn count_runtime_scene_entities(world: &mut World) -> usize {
    let mut query = world.query_filtered::<Entity, With<RuntimeSceneEntity>>();
    query.iter(world).count()
}

#[test]
fn gameplay_to_login_transition_cleans_runtime_state() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::state::app::StatesPlugin,
        bevy::asset::AssetPlugin::default(),
    ))
    .init_state::<AppState>()
    .add_message::<WorldRequest>()
    .add_plugins(SceneControllerPlugin::<LoginScene>::default())
    .add_plugins(SceneControllerPlugin::<GameplayScene>::default());

    transition_to(
        &mut app.world_mut().resource_mut::<NextState<AppState>>(),
        AppState::Gameplay,
    );
    app.update();

    assert!(app.world().contains_resource::<RuntimeSceneAssets>());
    assert!(count_runtime_scene_entities(app.world_mut()) >= 1);

    transition_to(
        &mut app.world_mut().resource_mut::<NextState<AppState>>(),
        AppState::Login,
    );
    app.update();

    assert!(!app.world().contains_resource::<RuntimeSceneAssets>());
    assert_eq!(count_runtime_scene_entities(app.world_mut()), 0);
}
