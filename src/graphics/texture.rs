use crate::math::*;
use glam::UVec2;

#[derive(Debug)]
pub struct GPUTexture {
    // shader uniforms
    pub texture: wgpu::Texture,
    pub sampler: wgpu::Sampler,
    pub view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,

    // me things
    pub size: UVec2,
}

pub struct TextureRaw {
    // shader uniforms
    pub texture: wgpu::Texture,
    pub sampler: wgpu::Sampler,
    pub view: wgpu::TextureView,
}

impl TextureRaw {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        camera_width: u32,
        camera_height: u32,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: camera_width * super::PIXEL_PREC,
            height: camera_height * super::PIXEL_PREC,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: -100.0, //TODO: Possible bug. May clip if too many sprites
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            sampler,
            view,
        }
    }
}
