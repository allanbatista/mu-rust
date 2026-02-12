use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_egui::EguiPlugin;

pub fn configure_character_viewer_app(app: &mut App, asset_root: String) {
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "MU Character Viewer".to_string(),
                    resolution: WindowResolution::new(1440, 900),
                    resizable: true,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: asset_root.into(),
                ..default()
            }),
    )
    .add_plugins(EguiPlugin::default());

    #[cfg(feature = "solari")]
    app.add_plugins(bevy::solari::SolariPlugins);
}
