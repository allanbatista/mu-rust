use std::marker::PhantomData;

use bevy::prelude::*;

pub trait SceneBuilder: Send + Sync + 'static {
    fn register(app: &mut App);
}

pub struct ScenePlugin<B: SceneBuilder> {
    _marker: PhantomData<B>,
}

impl<B: SceneBuilder> Default for ScenePlugin<B> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<B: SceneBuilder> Plugin for ScenePlugin<B> {
    fn build(&self, app: &mut App) {
        B::register(app);
    }
}

pub mod gameplay;
pub mod loading;
pub mod login;
