use bevy::math::Vec3;
use std::sync::OnceLock;

const WORLD_MIRROR_AXIS_ENV: &str = "MU_WORLD_MIRROR_AXIS";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldMirrorAxis {
    None,
    X,
    Z,
    XZ,
}

impl WorldMirrorAxis {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "none" | "off" | "0" => Some(Self::None),
            "x" | "horizontal" | "left-right" | "lr" => Some(Self::X),
            "z" | "vertical" | "top-bottom" | "tb" => Some(Self::Z),
            "xz" | "zx" | "both" | "xy" => Some(Self::XZ),
            _ => None,
        }
    }

    pub fn flips_handedness(self) -> bool {
        matches!(self, Self::X | Self::Z)
    }
}

/// Map-space mirror used to align world rendering with legacy client layout.
///
/// Default is `Z` (MU Y -> Bevy Z mirror). Override with `MU_WORLD_MIRROR_AXIS`:
/// - `none`
/// - `x`
/// - `z`
/// - `xz`
pub fn world_mirror_axis() -> WorldMirrorAxis {
    static AXIS: OnceLock<WorldMirrorAxis> = OnceLock::new();
    *AXIS.get_or_init(|| {
        std::env::var(WORLD_MIRROR_AXIS_ENV)
            .ok()
            .as_deref()
            .and_then(WorldMirrorAxis::parse)
            .unwrap_or(WorldMirrorAxis::Z)
    })
}

pub fn mirror_map_xz_with_axis(
    x: f32,
    z: f32,
    map_max_x: f32,
    map_max_z: f32,
    axis: WorldMirrorAxis,
) -> (f32, f32) {
    match axis {
        WorldMirrorAxis::None => (x, z),
        WorldMirrorAxis::X => (map_max_x - x, z),
        WorldMirrorAxis::Z => (x, map_max_z - z),
        WorldMirrorAxis::XZ => (map_max_x - x, map_max_z - z),
    }
}

pub fn mirror_map_position_with_axis(
    position: Vec3,
    map_max_x: f32,
    map_max_z: f32,
    axis: WorldMirrorAxis,
) -> Vec3 {
    let (x, z) = mirror_map_xz_with_axis(position.x, position.z, map_max_x, map_max_z, axis);
    Vec3::new(x, position.y, z)
}
