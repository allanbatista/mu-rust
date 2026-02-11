use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use serde_json::Value;
use std::sync::OnceLock;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct LegacyAdditiveMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub color_texture: Option<Handle<Image>>,
    #[uniform(2)]
    pub params: LegacyAdditiveParams,
}

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct LegacyAdditiveParams {
    pub intensity: f32,
    pub _padding: Vec3,
}

impl Default for LegacyAdditiveMaterial {
    fn default() -> Self {
        Self {
            color_texture: None,
            params: LegacyAdditiveParams {
                intensity: 1.0,
                _padding: Vec3::ZERO,
            },
        }
    }
}

impl Material for LegacyAdditiveMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/legacy_additive.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/legacy_additive.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

const LEGACY_ADDITIVE_INTENSITY_SCALE_ENV: &str = "MU_LEGACY_ADDITIVE_INTENSITY_SCALE";

fn legacy_additive_intensity_scale() -> f32 {
    static SCALE: OnceLock<f32> = OnceLock::new();
    *SCALE.get_or_init(|| {
        std::env::var(LEGACY_ADDITIVE_INTENSITY_SCALE_ENV)
            .ok()
            .and_then(|raw| raw.trim().parse::<f32>().ok())
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or(1.0)
    })
}

pub fn legacy_additive_intensity_from_extras(payload: &Value) -> f32 {
    let per_material = payload
        .get("mu_legacy_additive_intensity")
        .and_then(Value::as_f64)
        .map(|value| value as f32)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(1.0);

    per_material * legacy_additive_intensity_scale()
}

pub fn legacy_additive_from_standard(material: &StandardMaterial) -> LegacyAdditiveMaterial {
    let color_texture = material
        .base_color_texture
        .clone()
        .or_else(|| material.emissive_texture.clone());

    LegacyAdditiveMaterial {
        color_texture,
        params: LegacyAdditiveParams {
            intensity: 1.0,
            _padding: Vec3::ZERO,
        },
    }
}
