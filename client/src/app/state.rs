#[derive(bevy::prelude::States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    Loading,
    #[default]
    Login,
    Gameplay,
}
