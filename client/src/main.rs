mod bevy_compat;
mod character;
mod grid_overlay;
mod legacy_additive;
mod scene_runtime;
mod scenes;
mod settings;
mod ui;
mod world;

use bevy::app::PluginGroupBuilder;
use bevy::asset::AssetPlugin;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::state::app::AppExtStates;
use bevy::window::WindowResolution;
use bevy::winit::WinitSettings;
use bevy_egui::EguiPlugin;
use scene_runtime::registration::register_scene_runtime;
use scene_runtime::scene_loader::SceneLoaderPlugin;
use scenes::ScenePlugin;
use scenes::gameplay::GameplayScene;
use scenes::login::LoginScene;
use settings::{GameSettings, SettingsPlugin, SettingsResource};
use world::WorldPlugin;

#[derive(bevy::prelude::States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    Loading,
    #[default]
    Login,
    Gameplay,
}

fn main() {
    let startup_settings = settings::load_settings_or_default();
    if let Err(error) = settings::ensure_settings_file_exists(&startup_settings) {
        eprintln!(
            "Failed to ensure startup settings file '{}': {}",
            settings::SETTINGS_FILE_PATH,
            error
        );
    }

    let mut app = App::new();

    app.insert_resource(SettingsResource::new(startup_settings.clone()))
        .add_plugins(build_bevy_plugins(&startup_settings))
        .insert_resource(create_winit_settings(&startup_settings))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(EguiPlugin::default())
        .add_plugins(SceneLoaderPlugin)
        .add_plugins(WorldPlugin)
        .add_plugins(SettingsPlugin)
        .add_plugins(ui::HudPlugin)
        .init_state::<AppState>()
        .add_plugins(ScenePlugin::<LoginScene>::default())
        .add_plugins(ScenePlugin::<GameplayScene>::default());

    register_scene_runtime(&mut app);

    app.run();
}

fn build_bevy_plugins(startup_settings: &GameSettings) -> PluginGroupBuilder {
    DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some(create_window_settings(startup_settings)),
            ..Default::default()
        })
        .set(AssetPlugin {
            file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/../assets").into(),
            ..Default::default()
        })
        .disable::<PipelinedRenderingPlugin>()
}

fn create_winit_settings(startup_settings: &GameSettings) -> WinitSettings {
    let focused_mode = startup_settings.graphics.fps_limit.to_update_mode();
    WinitSettings {
        focused_mode,
        unfocused_mode: focused_mode,
    }
}

fn create_window_settings(startup_settings: &GameSettings) -> Window {
    Window {
        title: "Mu".into(),
        resolution: WindowResolution::new(
            startup_settings.graphics.resolution.width,
            startup_settings.graphics.resolution.height,
        ),
        resizable: true,
        mode: startup_settings.graphics.window_mode.to_bevy(),
        present_mode: settings::present_mode_for(&startup_settings.graphics),
        ..Default::default()
    }
}
