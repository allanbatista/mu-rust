#![allow(
    private_interfaces,
    clippy::collapsible_if,
    clippy::derivable_impls,
    clippy::doc_lazy_continuation,
    clippy::manual_find,
    clippy::manual_is_multiple_of,
    clippy::needless_update,
    clippy::reserve_after_initialization,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::useless_conversion
)]

pub mod app;
pub mod bevy_compat;
pub mod character;
pub mod composition;
pub mod core;
pub mod domain;
pub mod gameplay;
pub mod grid_overlay;
pub mod infra;
pub mod legacy_additive;
pub mod lightning_sprite_2d;
pub mod presentation;
pub mod scene_runtime;
pub mod settings;
pub mod ui;
pub mod world;

pub use app::state::AppState;
