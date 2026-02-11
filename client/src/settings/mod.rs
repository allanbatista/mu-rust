use crate::scene_runtime::systems::{RuntimeSunLight, SceneObjectDistanceCullingConfig};
use bevy::light::{
    CascadeShadowConfig, CascadeShadowConfigBuilder, DirectionalLightShadowMap,
    ShadowFilteringMethod,
};
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, PrimaryWindow, WindowMode, WindowResolution};
use bevy::winit::{UpdateMode, WinitSettings};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

pub const SETTINGS_FILE_PATH: &str = "./settings.yaml";

const RESOLUTION_PRESETS: [ResolutionSetting; 4] = [
    ResolutionSetting {
        width: 1280,
        height: 720,
    },
    ResolutionSetting {
        width: 1600,
        height: 900,
    },
    ResolutionSetting {
        width: 1920,
        height: 1080,
    },
    ResolutionSetting {
        width: 2560,
        height: 1440,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowModeSetting {
    Windowed,
    Fullscreen,
}

impl Default for WindowModeSetting {
    fn default() -> Self {
        Self::Windowed
    }
}

impl WindowModeSetting {
    pub const ALL: [Self; 2] = [Self::Windowed, Self::Fullscreen];

    pub fn next(self) -> Self {
        match self {
            Self::Windowed => Self::Fullscreen,
            Self::Fullscreen => Self::Windowed,
        }
    }

    pub fn to_bevy(self) -> WindowMode {
        match self {
            Self::Windowed => WindowMode::Windowed,
            Self::Fullscreen => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Windowed => "Windowed",
            Self::Fullscreen => "Fullscreen",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowQualitySetting {
    Off,
    Low,
    Medium,
    High,
}

impl Default for ShadowQualitySetting {
    fn default() -> Self {
        Self::Low
    }
}

impl ShadowQualitySetting {
    pub const ALL: [Self; 4] = [Self::Off, Self::Low, Self::Medium, Self::High];

    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FpsLimitSetting {
    Default60,
    Monitor,
    Unlimited,
}

impl Default for FpsLimitSetting {
    fn default() -> Self {
        Self::Default60
    }
}

impl FpsLimitSetting {
    pub const ALL: [Self; 3] = [Self::Default60, Self::Monitor, Self::Unlimited];

    pub fn next(self) -> Self {
        match self {
            Self::Default60 => Self::Monitor,
            Self::Monitor => Self::Unlimited,
            Self::Unlimited => Self::Default60,
        }
    }

    pub fn to_update_mode(self) -> UpdateMode {
        match self {
            Self::Default60 => UpdateMode::reactive(Duration::from_secs_f64(1.0 / 60.0)),
            Self::Monitor | Self::Unlimited => UpdateMode::Continuous,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Default60 => "60 FPS",
            Self::Monitor => "Monitor",
            Self::Unlimited => "Unlimited",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderDistanceSetting {
    Low,
    Medium,
    High,
    Ultra,
}

impl Default for RenderDistanceSetting {
    fn default() -> Self {
        Self::Medium
    }
}

impl RenderDistanceSetting {
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Ultra];

    pub fn next(self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Ultra,
            Self::Ultra => Self::Low,
        }
    }

    pub fn max_distance(self) -> f32 {
        match self {
            Self::Low => 2800.0,
            Self::Medium => 5000.0,
            Self::High => 8000.0,
            Self::Ultra => 12000.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Ultra => "Ultra",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ResolutionSetting {
    pub width: u32,
    pub height: u32,
}

impl Default for ResolutionSetting {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
        }
    }
}

impl ResolutionSetting {
    pub fn presets() -> &'static [Self] {
        &RESOLUTION_PRESETS
    }

    pub fn next(self) -> Self {
        let index = RESOLUTION_PRESETS
            .iter()
            .position(|preset| *preset == self)
            .unwrap_or(0);
        RESOLUTION_PRESETS[(index + 1) % RESOLUTION_PRESETS.len()]
    }

    pub fn label(self) -> String {
        format!("{}x{}", self.width, self.height)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsSettings {
    pub window_mode: WindowModeSetting,
    pub resolution: ResolutionSetting,
    pub shadow_quality: ShadowQualitySetting,
    pub vsync: bool,
    pub fps_limit: FpsLimitSetting,
    pub render_distance: RenderDistanceSetting,
    pub show_grass: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            window_mode: WindowModeSetting::Windowed,
            resolution: ResolutionSetting::default(),
            shadow_quality: ShadowQualitySetting::Low,
            vsync: true,
            fps_limit: FpsLimitSetting::Default60,
            render_distance: RenderDistanceSetting::Medium,
            show_grass: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    pub ambient_enabled: bool,
    pub effects_enabled: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            ambient_enabled: true,
            effects_enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Resource)]
#[serde(default)]
pub struct GameSettings {
    pub graphics: GraphicsSettings,
    pub audio: AudioSettings,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            graphics: GraphicsSettings::default(),
            audio: AudioSettings::default(),
        }
    }
}

#[derive(Debug, Error)]
pub enum SettingsIoError {
    #[error("failed to read settings file: {0}")]
    Read(std::io::Error),
    #[error("failed to write settings file: {0}")]
    Write(std::io::Error),
    #[error("failed to decode YAML settings: {0}")]
    Deserialize(serde_yaml::Error),
    #[error("failed to encode YAML settings: {0}")]
    Serialize(serde_yaml::Error),
}

#[derive(Resource, Clone)]
pub struct SettingsResource {
    pub current: GameSettings,
    path: PathBuf,
}

impl SettingsResource {
    pub fn new(current: GameSettings) -> Self {
        Self {
            current,
            path: PathBuf::from(SETTINGS_FILE_PATH),
        }
    }

    pub fn save_to_disk(&self) -> Result<(), SettingsIoError> {
        write_settings_to_path(&self.current, &self.path)
    }
}

#[derive(Resource, Clone, Debug)]
pub struct AudioCategoryState {
    pub ambient_enabled: bool,
    pub effects_enabled: bool,
}

impl Default for AudioCategoryState {
    fn default() -> Self {
        Self {
            ambient_enabled: true,
            effects_enabled: true,
        }
    }
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioCategoryState>()
            .add_systems(Update, apply_runtime_settings);
    }
}

pub fn load_settings_or_default() -> GameSettings {
    let path = Path::new(SETTINGS_FILE_PATH);

    if !path.exists() {
        return GameSettings::default();
    }

    match load_settings_from_path(path) {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!(
                "Failed to load settings from '{}': {}. Falling back to defaults.",
                SETTINGS_FILE_PATH, error
            );
            GameSettings::default()
        }
    }
}

pub fn ensure_settings_file_exists(settings: &GameSettings) -> Result<(), SettingsIoError> {
    let path = Path::new(SETTINGS_FILE_PATH);
    if path.exists() {
        return Ok(());
    }

    write_settings_to_path(settings, path)
}

pub fn present_mode_for(graphics: &GraphicsSettings) -> PresentMode {
    if matches!(graphics.fps_limit, FpsLimitSetting::Unlimited) {
        PresentMode::AutoNoVsync
    } else if graphics.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    }
}

fn load_settings_from_path(path: &Path) -> Result<GameSettings, SettingsIoError> {
    let raw = fs::read_to_string(path).map_err(SettingsIoError::Read)?;
    serde_yaml::from_str::<GameSettings>(&raw).map_err(SettingsIoError::Deserialize)
}

fn write_settings_to_path(settings: &GameSettings, path: &Path) -> Result<(), SettingsIoError> {
    let encoded = serde_yaml::to_string(settings).map_err(SettingsIoError::Serialize)?;
    fs::write(path, encoded).map_err(SettingsIoError::Write)
}

fn apply_runtime_settings(
    settings: Res<SettingsResource>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut winit_settings: ResMut<WinitSettings>,
    culling: Option<ResMut<SceneObjectDistanceCullingConfig>>,
    mut sun_query: Query<(&mut DirectionalLight, &mut CascadeShadowConfig), With<RuntimeSunLight>>,
    mut shadow_map: Option<ResMut<DirectionalLightShadowMap>>,
    camera_query: Query<Entity, With<Camera3d>>,
    added_sun_query: Query<(), Added<RuntimeSunLight>>,
    mut audio_categories: ResMut<AudioCategoryState>,
    mut commands: Commands,
    mut last_applied: Local<Option<GameSettings>>,
) {
    let runtime_sun_spawned = !added_sun_query.is_empty();
    if last_applied.as_ref() == Some(&settings.current) && !runtime_sun_spawned {
        return;
    }

    if let Ok(mut window) = windows.single_mut() {
        let target_mode = settings.current.graphics.window_mode.to_bevy();
        window.mode = target_mode;

        // In borderless fullscreen, forcing a custom logical resolution can
        // produce a top-left viewport offset. Keep monitor/native size there.
        if matches!(target_mode, WindowMode::Windowed) {
            window.resolution = WindowResolution::new(
                settings.current.graphics.resolution.width,
                settings.current.graphics.resolution.height,
            );
        }

        window.present_mode = present_mode_for(&settings.current.graphics);
    }

    let update_mode = settings.current.graphics.fps_limit.to_update_mode();
    winit_settings.focused_mode = update_mode;
    winit_settings.unfocused_mode = update_mode;

    if let Some(mut culling_config) = culling {
        let max_distance = settings.current.graphics.render_distance.max_distance();
        culling_config.enabled = max_distance > 0.0;
        culling_config.max_distance = max_distance;
        culling_config.max_distance_squared = max_distance * max_distance;
    }

    apply_shadow_quality(
        settings.current.graphics.shadow_quality,
        &mut sun_query,
        shadow_map.as_deref_mut(),
        &camera_query,
        &mut commands,
    );

    audio_categories.ambient_enabled = settings.current.audio.ambient_enabled;
    audio_categories.effects_enabled = settings.current.audio.effects_enabled;

    *last_applied = Some(settings.current.clone());
}

fn apply_shadow_quality(
    mode: ShadowQualitySetting,
    sun_query: &mut Query<(&mut DirectionalLight, &mut CascadeShadowConfig), With<RuntimeSunLight>>,
    shadow_map: Option<&mut DirectionalLightShadowMap>,
    camera_query: &Query<Entity, With<Camera3d>>,
    commands: &mut Commands,
) {
    let mut shadow_map = shadow_map;

    for (mut light, mut cascade_config) in sun_query.iter_mut() {
        match mode {
            ShadowQualitySetting::Low => {
                light.shadows_enabled = true;
                if let Some(map) = shadow_map.as_deref_mut() {
                    map.size = 1024;
                }
                *cascade_config = CascadeShadowConfigBuilder {
                    num_cascades: 1,
                    minimum_distance: 10.0,
                    maximum_distance: 8_000.0,
                    first_cascade_far_bound: 8_000.0,
                    overlap_proportion: 0.15,
                }
                .into();
            }
            ShadowQualitySetting::Medium => {
                light.shadows_enabled = true;
                if let Some(map) = shadow_map.as_deref_mut() {
                    map.size = 2048;
                }
                *cascade_config = CascadeShadowConfigBuilder {
                    num_cascades: 2,
                    minimum_distance: 10.0,
                    maximum_distance: 8_000.0,
                    first_cascade_far_bound: 1_200.0,
                    overlap_proportion: 0.15,
                }
                .into();
            }
            ShadowQualitySetting::High => {
                light.shadows_enabled = true;
                if let Some(map) = shadow_map.as_deref_mut() {
                    map.size = 4096;
                }
                *cascade_config = CascadeShadowConfigBuilder {
                    num_cascades: 3,
                    minimum_distance: 10.0,
                    maximum_distance: 8_000.0,
                    first_cascade_far_bound: 800.0,
                    overlap_proportion: 0.15,
                }
                .into();
            }
            ShadowQualitySetting::Off => {
                light.shadows_enabled = false;
            }
        }
    }

    if mode != ShadowQualitySetting::Off {
        let filtering = match mode {
            ShadowQualitySetting::Low => ShadowFilteringMethod::Hardware2x2,
            ShadowQualitySetting::Medium | ShadowQualitySetting::High => {
                ShadowFilteringMethod::Gaussian
            }
            ShadowQualitySetting::Off => return,
        };

        for entity in camera_query.iter() {
            commands.entity(entity).insert(filtering);
        }
    }
}
