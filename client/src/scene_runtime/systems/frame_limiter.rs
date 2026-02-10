use bevy::prelude::*;
use bevy::window::{PresentMode, WindowOccluded};
use bevy::winit::{UpdateMode, WinitSettings};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameLimitMode {
    Default,
    MonitorLimit,
    Disabled,
}

impl std::fmt::Display for FrameLimitMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameLimitMode::Default => write!(f, "60 FPS"),
            FrameLimitMode::MonitorLimit => write!(f, "VSync (monitor)"),
            FrameLimitMode::Disabled => write!(f, "Desativado"),
        }
    }
}

#[derive(Resource)]
pub struct DebugFrameLimiter {
    pub mode: FrameLimitMode,
}

impl Default for DebugFrameLimiter {
    fn default() -> Self {
        Self {
            mode: FrameLimitMode::Default,
        }
    }
}

pub fn cycle_debug_frame_limit(
    keys: Res<ButtonInput<KeyCode>>,
    mut limiter: ResMut<DebugFrameLimiter>,
    mut windows: Query<&mut Window>,
    mut winit_settings: ResMut<WinitSettings>,
) {
    if !keys.just_pressed(KeyCode::F4) {
        return;
    }

    limiter.mode = match limiter.mode {
        FrameLimitMode::Default => FrameLimitMode::MonitorLimit,
        FrameLimitMode::MonitorLimit => FrameLimitMode::Disabled,
        FrameLimitMode::Disabled => FrameLimitMode::Default,
    };

    if let Ok(mut window) = windows.single_mut() {
        match limiter.mode {
            FrameLimitMode::Default => {
                window.present_mode = PresentMode::AutoVsync;
                let mode = UpdateMode::reactive(Duration::from_secs_f64(1.0 / 60.0));
                winit_settings.focused_mode = mode;
                winit_settings.unfocused_mode = mode;
            }
            FrameLimitMode::MonitorLimit => {
                window.present_mode = PresentMode::AutoVsync;
                winit_settings.focused_mode = UpdateMode::Continuous;
                winit_settings.unfocused_mode = UpdateMode::Continuous;
            }
            FrameLimitMode::Disabled => {
                window.present_mode = PresentMode::AutoNoVsync;
                winit_settings.focused_mode = UpdateMode::Continuous;
                winit_settings.unfocused_mode = UpdateMode::Continuous;
            }
        }
    }

    info!("Frame limit: {}", limiter.mode);
}

pub fn handle_window_occlusion(
    mut events: MessageReader<WindowOccluded>,
    mut winit_settings: ResMut<WinitSettings>,
) {
    for event in events.read() {
        if event.occluded {
            // Minimized/hidden — drop to 5fps
            winit_settings.unfocused_mode =
                UpdateMode::reactive_low_power(Duration::from_millis(200));
        } else {
            // Visible again — match focused_mode
            winit_settings.unfocused_mode = winit_settings.focused_mode;
        }
    }
}
