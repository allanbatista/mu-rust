pub mod bevy_compat;
pub mod grid_overlay;
pub mod legacy_additive;
pub mod scene_runtime;
pub mod settings;

#[derive(bevy::prelude::States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    Loading,
    #[default]
    Login,
    Gameplay,
}
