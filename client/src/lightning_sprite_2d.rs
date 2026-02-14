use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState,
    RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey};

const BLEND_ADDITIVE: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    },
};

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct LightningSprite2dMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub color_texture: Option<Handle<Image>>,
    #[uniform(2)]
    pub params: LightningSprite2dParams,
}

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct LightningSprite2dParams {
    pub intensity: f32,
    pub _padding: Vec3,
}

impl Default for LightningSprite2dMaterial {
    fn default() -> Self {
        Self {
            color_texture: None,
            params: LightningSprite2dParams {
                intensity: 1.0,
                _padding: Vec3::ZERO,
            },
        }
    }
}

impl Material2d for LightningSprite2dMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/lightning_sprite_2d.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = &mut descriptor.fragment {
            if let Some(target_state) = &mut fragment.targets[0] {
                target_state.blend = Some(BLEND_ADDITIVE);
            }
        }
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
