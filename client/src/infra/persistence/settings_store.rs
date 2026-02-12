use crate::domain::settings::{GameSettings, SettingsIoError};

pub fn load() -> GameSettings {
    crate::settings::load_settings_or_default()
}

pub fn ensure_exists(settings: &GameSettings) -> Result<(), SettingsIoError> {
    crate::settings::ensure_settings_file_exists(settings)
}
