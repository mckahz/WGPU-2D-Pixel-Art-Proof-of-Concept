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
