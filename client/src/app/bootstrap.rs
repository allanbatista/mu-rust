use bevy::prelude::App;

use crate::composition::client_runtime::configure_client_app;
use crate::domain::settings::GameSettings;
use crate::infra::persistence::settings_store;

pub fn run_client_app() {
    let startup_settings = load_startup_settings();
    let mut app = App::new();
    configure_client_app(&mut app, &startup_settings);
    app.run();
}

fn load_startup_settings() -> GameSettings {
    let startup_settings = settings_store::load();
    if let Err(error) = settings_store::ensure_exists(&startup_settings) {
        eprintln!(
            "Failed to ensure startup settings file '{}': {}",
            crate::settings::SETTINGS_FILE_PATH,
            error
        );
    }
    startup_settings
}
