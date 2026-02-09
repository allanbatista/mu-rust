mod scene_runtime;
mod scenes;
mod world;

use bevy::app::PluginGroupBuilder;
use bevy::asset::AssetPlugin;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::state::app::AppExtStates;
use bevy::window::WindowResolution;
use scene_runtime::scene_loader::SceneLoaderPlugin;
use scenes::ScenePlugin;
use scenes::loading::LoadingScene;
use scenes::login::LoginScene;
use world::WorldPlugin;

#[derive(bevy::prelude::States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    Loading,
    Mock,
}

fn main() {
    App::new()
        .add_plugins(build_bevy_plugins())
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(SceneLoaderPlugin)
        .add_plugins(WorldPlugin)
        .init_state::<AppState>()
        .add_plugins(ScenePlugin::<LoadingScene>::default())
        .add_plugins(ScenePlugin::<LoginScene>::default())
        .run();
}

fn build_bevy_plugins() -> PluginGroupBuilder {
    DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some(create_window_settings()),
            ..Default::default()
        })
        .set(AssetPlugin {
            file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/../assets").into(),
            ..Default::default()
        })
}

fn create_window_settings() -> Window {
    Window {
        title: "Mu".into(),
        resolution: WindowResolution::new(1280.0, 720.0),
        resizable: false,
        ..Default::default()
    }
}
