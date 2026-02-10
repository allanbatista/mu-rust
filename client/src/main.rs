mod bevy_compat;
mod character;
mod scene_runtime;
mod scenes;
mod world;

use bevy::app::PluginGroupBuilder;
use bevy::asset::AssetPlugin;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::state::app::AppExtStates;
use bevy::window::{PresentMode, WindowResolution};
use bevy::winit::{UpdateMode, WinitSettings};
use scene_runtime::scene_loader::SceneLoaderPlugin;
use scenes::ScenePlugin;
use scenes::loading::LoadingScene;
use scenes::login::LoginScene;
use std::time::Duration;
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
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::reactive(Duration::from_secs_f64(1.0 / 60.0)),
            unfocused_mode: UpdateMode::reactive(Duration::from_secs_f64(1.0 / 60.0)),
        })
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
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
        .disable::<PipelinedRenderingPlugin>()
}

fn create_window_settings() -> Window {
    Window {
        title: "Mu".into(),
        resolution: WindowResolution::new(1280, 720),
        resizable: false,
        present_mode: PresentMode::AutoVsync,
        ..Default::default()
    }
}
