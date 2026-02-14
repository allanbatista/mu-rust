use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;
use bevy::window::WindowResolution;
use bevy_egui::EguiPlugin;

use crate::infra::assets::configure_asset_resolver;
use crate::lightning_sprite_2d::LightningSprite2dMaterial;

pub fn configure_character_viewer_app(
    app: &mut App,
    asset_root: String,
    use_remaster_assets: bool,
) {
    configure_asset_resolver(asset_root.clone(), use_remaster_assets);

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
    .add_plugins(EguiPlugin::default())
    .add_plugins(Material2dPlugin::<LightningSprite2dMaterial>::default());

    #[cfg(feature = "solari")]
    app.add_plugins(bevy::solari::SolariPlugins);
}
