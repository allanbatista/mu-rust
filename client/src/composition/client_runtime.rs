use bevy::prelude::*;
use bevy::state::app::AppExtStates;
use bevy_egui::EguiPlugin;

use crate::AppState;
use crate::app::plugins::{build_bevy_plugins, create_winit_settings};
use crate::domain::settings::{GameSettings, SettingsPlugin, SettingsResource};
use crate::gameplay::controllers::scene_controller::SceneControllerPlugin;
use crate::gameplay::runtime::registration::register_gameplay_runtime;
use crate::gameplay::scenes::gameplay::GameplayScene;
use crate::gameplay::scenes::login::LoginScene;
use crate::presentation::ui::hud::HudPresentationPlugin;
use crate::scene_runtime::scene_loader::SceneLoaderPlugin;
use crate::world::WorldPlugin;

pub fn configure_client_app(app: &mut App, startup_settings: &GameSettings) {
    app.insert_resource(SettingsResource::new(startup_settings.clone()))
        .add_plugins(build_bevy_plugins(startup_settings))
        .insert_resource(create_winit_settings(startup_settings))
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(EguiPlugin::default())
        .add_plugins(SceneLoaderPlugin)
        .add_plugins(WorldPlugin)
        .add_plugins(SettingsPlugin)
        .add_plugins(HudPresentationPlugin)
        .init_state::<AppState>()
        .add_plugins(SceneControllerPlugin::<LoginScene>::default())
        .add_plugins(SceneControllerPlugin::<GameplayScene>::default());

    register_gameplay_runtime(app);
}
