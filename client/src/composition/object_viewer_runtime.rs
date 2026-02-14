use bevy::asset::AssetPlugin;
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_egui::EguiPlugin;

use crate::infra::assets::configure_asset_resolver;
use crate::legacy_additive::LegacyAdditiveMaterial;

pub fn configure_object_animation_viewer_app(
    app: &mut App,
    asset_root: String,
    use_remaster_assets: bool,
) {
    configure_asset_resolver(asset_root.clone(), use_remaster_assets);

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "MU Object Animation Viewer".to_string(),
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
    .add_plugins(MaterialPlugin::<LegacyAdditiveMaterial>::default())
    .add_plugins(EguiPlugin::default());

    #[cfg(feature = "solari")]
    app.add_plugins(bevy::solari::SolariPlugins);
}
