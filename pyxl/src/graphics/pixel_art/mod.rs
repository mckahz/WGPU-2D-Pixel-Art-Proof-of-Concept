pub mod sprite;
pub mod sprite_sheet;
pub mod texture;
pub mod tilemap;

use glam::*;
use sprite::*;
use texture::*;
use wgpu::util::DeviceExt;

use crate::math::extend3d_to_uvec2;

pub const PIXEL_PREC: u32 = 5;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera {
    offset: [f32; 2],
}
impl From<Vec2> for Camera {
    fn from(offset: Vec2) -> Self {
        Self {
            offset: offset.to_array(),
        }
    }
}

pub struct GPUWorldToPixel {
    pub raw: WorldToPixel,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WorldToPixel {
    pub scale: [f32; 2],
    pub offset: [f32; 2],
}

impl WorldToPixel {
    pub fn new(camera_width: u32, camera_height: u32) -> Self {
        Self {
            scale: [2.0 / (camera_width as f32), -2.0 / (camera_height as f32)],
            offset: [camera_width as f32 * -0.5, camera_height as f32 * -0.5],
        }
    }
}

pub struct PixelArt {
    pub w2p: GPUWorldToPixel,
    pub texture: GPUTexture,
    pub depth_texture: TextureRaw,

    pub sprite_shader: wgpu::ShaderModule,
    pub sprite_sheet_shader: wgpu::ShaderModule,
    pub tile_layer_shader: wgpu::ShaderModule,
    pub sprite_pipeline_layout: wgpu::PipelineLayout,
    pub sprite_render_pipeline: wgpu::RenderPipeline,
    pub sprite_sheet_pipeline_layout: wgpu::PipelineLayout,
    pub sprite_sheet_render_pipeline: wgpu::RenderPipeline,
    pub tile_layer_pipeline_layout: wgpu::PipelineLayout,
    pub tile_layer_render_pipeline: wgpu::RenderPipeline,

    pub camera: Camera,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,

    pub tile_depth_bind_group_layout: wgpu::BindGroupLayout,

    pub sprite_sheet_data_bind_group_layout: wgpu::BindGroupLayout,
}

impl PixelArt {
    pub fn new(
        device: &wgpu::Device,
        camera_width: u32,
        camera_height: u32,
        config: &wgpu::SurfaceConfiguration,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let camera = Camera::from(00.0 * Vec2::ONE);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[camera]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let w2p = {
            let raw = WorldToPixel::new(camera_width, camera_height);
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[raw]),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            });
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
            GPUWorldToPixel {
                raw,
                buffer,
                bind_group,
                bind_group_layout,
            }
        };

        let texture = {
            let size = wgpu::Extent3d {
                width: PIXEL_PREC * camera_width,
                height: PIXEL_PREC * camera_height,
                depth_or_array_layers: 1,
            };
            let render_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
            });
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            let view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

            GPUTexture {
                texture: render_texture,
                sampler,
                view,
                bind_group,
                size: extend3d_to_uvec2(&size),
            }
        };

        let depth_texture = TextureRaw::create_depth_texture(&device, camera_width, camera_height);

        let sprite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
        });

        let sprite_sheet_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite_sheet.wgsl").into()),
        });

        let tile_layer_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("tile_layer.wgsl").into()),
        });

        let sprite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &w2p.bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let sprite_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&sprite_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &sprite_shader,
                    entry_point: "vs_main",
                    buffers: &[sprite::Vertex::desc(), SpriteInstance::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &sprite_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::OVER,
                            alpha: wgpu::BlendComponent::OVER,
                        }),
                        write_mask: wgpu::ColorWrites::COLOR,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: TextureRaw::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

        let sprite_sheet_data_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let sprite_sheet_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &w2p.bind_group_layout,
                    &camera_bind_group_layout,
                    &sprite_sheet_data_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let sprite_sheet_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&sprite_sheet_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &sprite_sheet_shader,
                    entry_point: "vs_main",
                    buffers: &[
                        sprite_sheet::Vertex::desc(),
                        sprite_sheet::SpriteSheetInstance::desc(),
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &sprite_sheet_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::OVER,
                            alpha: wgpu::BlendComponent::OVER,
                        }),
                        write_mask: wgpu::ColorWrites::COLOR,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: TextureRaw::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

        let tile_depth_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let tile_layer_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &w2p.bind_group_layout,
                    &camera_bind_group_layout,
                    &tile_depth_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let tile_layer_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&tile_layer_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &tile_layer_shader,
                    entry_point: "vs_main",
                    buffers: &[tilemap::Vertex::desc(), tilemap::TileInstance::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &tile_layer_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::OVER,
                            alpha: wgpu::BlendComponent::OVER,
                        }),
                        write_mask: wgpu::ColorWrites::COLOR,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: TextureRaw::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

        Self {
            texture,
            depth_texture,
            w2p,
            sprite_pipeline_layout,
            sprite_sheet_pipeline_layout,
            camera,
            camera_buffer,
            camera_bind_group,
            sprite_shader,
            sprite_sheet_shader,
            sprite_render_pipeline,
            sprite_sheet_render_pipeline,
            tile_layer_pipeline_layout,
            tile_layer_render_pipeline,
            tile_layer_shader,
            tile_depth_bind_group_layout,
            sprite_sheet_data_bind_group_layout,
        }
    }
}
