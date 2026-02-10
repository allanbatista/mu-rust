use bevy::camera::{Camera2d, Camera3d, Projection};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::CascadeShadowConfig;
use bevy::mesh::Mesh3d;
use bevy::pbr::{Material, MeshMaterial3d};
use bevy::prelude::*;
use bevy::scene::SceneRoot;
use bevy::text::{Justify, TextColor, TextFont, TextLayout};
use bevy::ui::{Node, PositionType, Val};

#[derive(Bundle, Clone)]
pub struct SpatialBundle {
    pub transform: Transform,
    pub visibility: Visibility,
}

impl Default for SpatialBundle {
    fn default() -> Self {
        Self {
            transform: Transform::default(),
            visibility: Visibility::Visible,
        }
    }
}

#[derive(Bundle, Clone)]
pub struct SceneBundle {
    pub scene: SceneRoot,
    pub transform: Transform,
    pub visibility: Visibility,
}

impl Default for SceneBundle {
    fn default() -> Self {
        Self {
            scene: SceneRoot(Handle::default()),
            transform: Transform::default(),
            visibility: Visibility::Visible,
        }
    }
}

#[derive(Bundle, Clone)]
pub struct DirectionalLightBundle {
    pub directional_light: DirectionalLight,
    pub cascade_shadow_config: CascadeShadowConfig,
    pub transform: Transform,
    pub visibility: Visibility,
}

impl Default for DirectionalLightBundle {
    fn default() -> Self {
        Self {
            directional_light: DirectionalLight::default(),
            cascade_shadow_config: CascadeShadowConfig::default(),
            transform: Transform::default(),
            visibility: Visibility::Visible,
        }
    }
}

#[derive(Bundle, Clone)]
pub struct PointLightBundle {
    pub point_light: PointLight,
    pub transform: Transform,
    pub visibility: Visibility,
}

impl Default for PointLightBundle {
    fn default() -> Self {
        Self {
            point_light: PointLight::default(),
            transform: Transform::default(),
            visibility: Visibility::Visible,
        }
    }
}

#[derive(Bundle, Clone)]
pub struct Camera3dBundle {
    pub camera: Camera,
    pub camera_3d: Camera3d,
    pub projection: Projection,
    pub tonemapping: Tonemapping,
    pub transform: Transform,
}

impl Default for Camera3dBundle {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            camera_3d: Camera3d::default(),
            projection: Projection::default(),
            tonemapping: Tonemapping::None,
            transform: Transform::default(),
        }
    }
}

#[derive(Bundle, Clone)]
pub struct Camera2dBundle {
    pub camera: Camera,
    pub camera_2d: Camera2d,
    pub tonemapping: Tonemapping,
    pub transform: Transform,
}

impl Default for Camera2dBundle {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            camera_2d: Camera2d,
            tonemapping: Tonemapping::None,
            transform: Transform::default(),
        }
    }
}

#[derive(Bundle, Clone)]
pub struct PbrBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub visibility: Visibility,
}

impl Default for PbrBundle {
    fn default() -> Self {
        Self {
            mesh: Mesh3d(Handle::default()),
            material: MeshMaterial3d(Handle::default()),
            transform: Transform::default(),
            visibility: Visibility::Visible,
        }
    }
}

#[derive(Bundle, Clone)]
pub struct MaterialMeshBundle<M: Material> {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<M>,
    pub transform: Transform,
    pub visibility: Visibility,
}

impl<M: Material> Default for MaterialMeshBundle<M> {
    fn default() -> Self {
        Self {
            mesh: Mesh3d(Handle::default()),
            material: MeshMaterial3d(Handle::default()),
            transform: Transform::default(),
            visibility: Visibility::Visible,
        }
    }
}

#[derive(Clone)]
pub struct TextStyle {
    pub font: Handle<Font>,
    pub font_size: f32,
    pub color: Color,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: Handle::default(),
            font_size: 16.0,
            color: Color::WHITE,
        }
    }
}

#[derive(Clone)]
pub struct Style {
    pub position_type: PositionType,
    pub top: Val,
    pub right: Val,
    pub bottom: Val,
    pub left: Val,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            position_type: PositionType::Relative,
            top: Val::Auto,
            right: Val::Auto,
            bottom: Val::Auto,
            left: Val::Auto,
        }
    }
}

#[derive(Clone, Copy)]
pub enum JustifyText {
    Left,
    Center,
    Right,
}

impl From<JustifyText> for Justify {
    fn from(value: JustifyText) -> Self {
        match value {
            JustifyText::Left => Justify::Left,
            JustifyText::Center => Justify::Center,
            JustifyText::Right => Justify::Right,
        }
    }
}

#[derive(Bundle, Clone)]
pub struct TextBundle {
    pub text: Text,
    pub text_font: TextFont,
    pub text_color: TextColor,
    pub text_layout: TextLayout,
    pub node: Node,
    pub background_color: BackgroundColor,
}

impl Default for TextBundle {
    fn default() -> Self {
        Self {
            text: Text::new(""),
            text_font: TextFont::default(),
            text_color: TextColor(Color::WHITE),
            text_layout: TextLayout::default(),
            node: Node::default(),
            background_color: BackgroundColor::default(),
        }
    }
}

impl TextBundle {
    pub fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self {
            text: Text::new(value),
            text_font: TextFont {
                font: style.font,
                font_size: style.font_size,
                ..default()
            },
            text_color: TextColor(style.color),
            ..default()
        }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.node.position_type = style.position_type;
        self.node.top = style.top;
        self.node.right = style.right;
        self.node.bottom = style.bottom;
        self.node.left = style.left;
        self
    }

    pub fn with_text_justify(mut self, justify: JustifyText) -> Self {
        self.text_layout = TextLayout::new_with_justify(justify.into());
        self
    }
}
