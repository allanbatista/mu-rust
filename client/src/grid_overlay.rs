use bevy::prelude::*;

pub const GRID_OVERLAY_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.25);

#[derive(Clone, Copy, Debug)]
pub struct GridOverlayConfig {
    pub cell_size: f32,
    pub visible_half_cells: i32,
    pub y_offset: f32,
    pub color: Color,
}

impl Default for GridOverlayConfig {
    fn default() -> Self {
        Self {
            cell_size: 100.0,
            visible_half_cells: 25,
            y_offset: 0.25,
            color: GRID_OVERLAY_COLOR,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GridLineSegment {
    pub start: Vec3,
    pub end: Vec3,
}

pub fn grid_line_count(visible_half_cells: i32) -> usize {
    let half = visible_half_cells.max(0) as usize;
    if half == 0 {
        return 0;
    }
    let line_count = 2 * half + 1;
    let segments_per_line = 2 * half;
    line_count * segments_per_line * 2
}

pub fn build_grid_segments<F>(
    center: Vec3,
    config: GridOverlayConfig,
    mut sample_height: F,
) -> Vec<GridLineSegment>
where
    F: FnMut(f32, f32) -> f32,
{
    if config.cell_size <= 0.0 {
        return Vec::new();
    }

    let half = config.visible_half_cells.max(0);
    if half == 0 {
        return Vec::new();
    }
    let cx = (center.x / config.cell_size).round() * config.cell_size;
    let cz = (center.z / config.cell_size).round() * config.cell_size;
    let mut lines = Vec::with_capacity(grid_line_count(half));

    for i in -half..=half {
        let delta = i as f32 * config.cell_size;

        let z_world = cz + delta;
        for segment_step in -half..half {
            let x_start = cx + segment_step as f32 * config.cell_size;
            let x_end = x_start + config.cell_size;
            let y_start = sample_height(x_start, z_world) + config.y_offset;
            let y_end = sample_height(x_end, z_world) + config.y_offset;
            lines.push(GridLineSegment {
                start: Vec3::new(x_start, y_start, z_world),
                end: Vec3::new(x_end, y_end, z_world),
            });
        }

        let x_world = cx + delta;
        for segment_step in -half..half {
            let z_start = cz + segment_step as f32 * config.cell_size;
            let z_end = z_start + config.cell_size;
            let y_start = sample_height(x_world, z_start) + config.y_offset;
            let y_end = sample_height(x_world, z_end) + config.y_offset;
            lines.push(GridLineSegment {
                start: Vec3::new(x_world, y_start, z_start),
                end: Vec3::new(x_world, y_end, z_end),
            });
        }
    }

    lines
}

pub fn segment_transform(segment: GridLineSegment, thickness: f32) -> Option<Transform> {
    let direction = segment.end - segment.start;
    let length = direction.length();
    if length <= f32::EPSILON {
        return None;
    }
    let midpoint = (segment.start + segment.end) * 0.5;
    let rotation = Quat::from_rotation_arc(Vec3::X, direction / length);
    Some(Transform {
        translation: midpoint,
        rotation,
        scale: Vec3::new(length, thickness.max(0.01), thickness.max(0.01)),
    })
}

pub fn draw_grid_overlay<F>(
    gizmos: &mut Gizmos,
    center: Vec3,
    config: GridOverlayConfig,
    sample_height: F,
) where
    F: FnMut(f32, f32) -> f32,
{
    let segments = build_grid_segments(center, config, sample_height);
    for segment in segments {
        gizmos.line(segment.start, segment.end, config.color);
    }
}
