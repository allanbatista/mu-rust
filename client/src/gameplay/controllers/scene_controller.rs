use bevy::prelude::*;

use crate::AppState;

pub trait SceneController: Send + Sync + 'static {
    fn register(app: &mut App);

    fn scene_id() -> SceneId {
        SceneId::Unknown
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SceneId {
    Loading,
    Login,
    Gameplay,
    Unknown,
}

pub struct SceneControllerPlugin<C: SceneController> {
    _marker: std::marker::PhantomData<C>,
}

impl<C: SceneController> Default for SceneControllerPlugin<C> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<C: SceneController> Plugin for SceneControllerPlugin<C> {
    fn build(&self, app: &mut App) {
        C::register(app);
    }
}

pub fn transition_to(next_state: &mut NextState<AppState>, target: AppState) {
    next_state.set(target);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::AppExtStates;

    #[test]
    fn transition_to_updates_state_machine() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin))
            .init_state::<AppState>();

        transition_to(
            &mut app.world_mut().resource_mut::<NextState<AppState>>(),
            AppState::Gameplay,
        );

        app.update();

        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::Gameplay
        );
    }
}
