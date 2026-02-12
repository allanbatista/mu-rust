use bevy::prelude::*;

use client::gameplay::controllers::scene_controller::{SceneController, SceneControllerPlugin};

#[derive(Resource, Default)]
struct Marker(bool);

struct DummySceneController;

impl SceneController for DummySceneController {
    fn register(app: &mut App) {
        app.insert_resource(Marker(true));
    }
}

#[test]
fn scene_controller_plugin_registers_controller_systems() {
    let mut app = App::new();
    app.add_plugins(SceneControllerPlugin::<DummySceneController>::default());

    assert!(app.world().resource::<Marker>().0);
}
