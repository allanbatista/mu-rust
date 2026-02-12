use bevy::prelude::*;

pub struct HudPresentationPlugin;

impl Plugin for HudPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::ui::HudPlugin);
    }
}
