use bevy::app::PluginGroupBuilder;
use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::window::WindowResolution;
use bevy::winit::WinitSettings;

use crate::settings::{self, GameSettings};

pub fn build_bevy_plugins(startup_settings: &GameSettings) -> PluginGroupBuilder {
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

pub fn create_winit_settings(startup_settings: &GameSettings) -> WinitSettings {
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
