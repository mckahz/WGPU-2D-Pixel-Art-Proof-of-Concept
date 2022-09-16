pub mod gpu_texture;
pub mod sprite;
pub mod vertex;

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    iter,
    rc::Rc,
};
use wgpu::util::DeviceExt;

use crate::{
    graphics::{gpu_texture::*, sprite::*, vertex::*},
    math::extend3d_to_uvec2,
};
use glam::*;
use winit::{dpi::PhysicalSize, window::Window};

const PIXEL_PREC: u32 = 5;

#[derive(Debug)]
pub struct DrawParams {
    pub position: Vec2,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl Default for DrawParams {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            flip_x: false,
            flip_y: false,
        }
    }
}

impl DrawParams {
    pub fn from_pos(position: Vec2) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }
}

pub enum DrawJob<'a> {
    Sprite(&'a Sprite, DrawParams),
    SpriteSheet(&'a SpriteSheet, DrawParams),
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

pub struct Downscaling {
    pub vertex_buffer: wgpu::Buffer,
    pub shader: wgpu::ShaderModule,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub render_pipeline: wgpu::RenderPipeline,
}

impl Downscaling {
    fn vertices(
        screen_width: u32,
        screen_height: u32,
        camera_width: u32,
        camera_height: u32,
    ) -> [Vertex; 4] {
        let (sw, sh) = (screen_width as f32, screen_height as f32);
        let (cw, ch) = (camera_width as f32, camera_height as f32);
        let width_r = sw / cw;
        let height_r = sh / ch;
        let half_width = if width_r > height_r {
            (sh / sw) * (cw / ch)
        } else {
            1.0
        };
        let half_height = if width_r < height_r {
            (sw / sh) * (ch / cw)
        } else {
            1.0
        };

        [
            Vertex {
                position: [-half_width, half_height, 0.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [-half_width, -half_height, 0.0],
                tex_coords: [0.0, 1.0],
            },
            Vertex {
                position: [half_width, half_height, 0.0],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [half_width, -half_height, 0.0],
                tex_coords: [1.0, 1.0],
            },
        ]
    }
}

pub struct Upscaling {
    pub w2p: GPUWorldToPixel,
    pub texture: GPUTexture,
    pub shader: wgpu::ShaderModule,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub render_pipeline: wgpu::RenderPipeline,
    pub depth_texture: GPUTexture,
}

impl Upscaling {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        camera_width: u32,
        camera_height: u32,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> GPUTexture {
        let size = wgpu::Extent3d {
            width: camera_width * PIXEL_PREC,
            height: camera_height * PIXEL_PREC,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: None,
            lod_min_clamp: -100.0, //TODO: Possible bug. May clip if too many sprites
            lod_max_clamp: 100.0,
            ..Default::default()
        });

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
            texture,
            sampler,
            bind_group,
            view,
            size: extend3d_to_uvec2(&size),
        }
    }
}

pub struct Renderer {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,

    pub downscaling: Downscaling,
    pub upscaling: Upscaling,

    pub camera_width: u32,
    pub camera_height: u32,

    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub sprite_ids: Rc<RefCell<HashSet<usize>>>,

    pub quad_index_buffer: wgpu::Buffer,
}

impl Renderer {
    pub const INDICES: [u16; 6] = [1, 2, 0, 2, 1, 3];

    pub fn load_texture(&mut self, path: &str) -> Option<GPUTexture> {
        use std::path::PathBuf;
        let mut pwd = PathBuf::from("./assets/");
        pwd.push(path);
        let img = image::load_from_memory(&std::fs::read(pwd).unwrap()).unwrap();
        let rgba = {
            use image::*;
            let top = img.to_rgba8();
            let top_d = img.dimensions();
            let mut bottom = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(
                top_d.0 + 2,
                top_d.1 + 2,
                Rgba([0, 0, 0, 0]),
            ));
            imageops::overlay(&mut bottom, &top, 1, 1);
            match bottom {
                DynamicImage::ImageRgba8(a) => a,
                _ => panic!(),
            };
            top
        };
        let dimensions = rgba.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture_with_data(
            &self.queue,
            &wgpu::TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            },
            &rgba,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.texture_bind_group_layout,
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

        Some(GPUTexture {
            bind_group,
            texture,
            view,
            size: extend3d_to_uvec2(&size),
            sampler,
        })
    }

    pub fn load_sprite(&mut self, origin: Origin, path: &str) -> Option<Sprite> {
        let texture = self.load_texture(path).unwrap();

        let (width, height) = texture.size.as_vec2().into();
        let vertices = [
            Vertex {
                position: [0.0, 0.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [0.0, height, 0.0],
                tex_coords: [0.0, 1.0],
            },
            Vertex {
                position: [width, 0.0, 0.0],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [width, height, 0.0],
                tex_coords: [1.0, 1.0],
            },
        ];

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let ids = self.sprite_ids.clone();
        let id = {
            let mut id = 0;
            loop {
                if !ids.as_ref().borrow().contains(&id) {
                    ids.borrow_mut().insert(id);
                    break id;
                }
                id += 1;
            }
        };

        Some(Sprite {
            vertex_buffer,
            texture,
            origin,
            id,
            ids,
        })
    }

    pub fn load_sprite_sheet(
        &mut self,
        origin: Origin,
        path: &str,
        count: u8,
        frame_rate: FrameRate,
        orientation: Orientation,
    ) -> Option<SpriteSheet> {
        let sprite = self.load_sprite(origin, path)?;
        Some(SpriteSheet {
            sprite,
            count,
            t: 0.0,
            frame_rate,
            orientation,
        })
    }

    pub async fn new(window: &Window, camera_width: u32, camera_height: u32) -> Renderer {
        let size = window.inner_size().clone();

        // ------------------------------------------------------------------------------------------- Wgpu Initialization
        let (_, surface, _, device, queue, config) = {
            // The instance is a handle to our GPU
            // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
            let instance = wgpu::Instance::new(wgpu::Backends::all());
            let surface = unsafe { instance.create_surface(window) };
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .unwrap();
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        features: wgpu::Features::empty(),
                        // WebGL doesn't support all of wgpu's features, so if
                        // we're building for the web we'll have to disable some.
                        limits: if cfg!(target_arch = "wasm32") {
                            wgpu::Limits::downlevel_webgl2_defaults()
                        } else {
                            wgpu::Limits::default()
                        },
                    },
                    None, // Trace path
                )
                .await
                .unwrap();

            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface.get_supported_formats(&adapter)[0],
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo,
            };
            surface.configure(&device, &config);

            (instance, surface, adapter, device, queue, config)
        };

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: None,
            });

        // ------------------------------------------------------------------------------------------- Downscaling
        let downscaling = {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(include_str!("downscaling.wgsl").into()),
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

            let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[Vertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::OVER,
                            alpha: wgpu::BlendComponent::OVER,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
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
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

            let vertices =
                Downscaling::vertices(size.width, size.height, camera_width, camera_height);
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
            });

            Downscaling {
                vertex_buffer,
                shader,
                pipeline_layout,
                render_pipeline,
            }
        };

        // ------------------------------------------------------------------------------------------- Upscaling
        let upscaling = {
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
                let texture = device.create_texture(&wgpu::TextureDescriptor {
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
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &texture_bind_group_layout,
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
                    texture,
                    sampler,
                    view,
                    bind_group,
                    size: extend3d_to_uvec2(&size),
                }
            };

            let texture_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: None,
                });

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(include_str!("upscaling.wgsl").into()),
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&texture_bind_group_layout, &w2p.bind_group_layout],
                push_constant_ranges: &[],
            });

            let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[Vertex::desc(), Instance::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::OVER,
                            alpha: wgpu::BlendComponent::OVER,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
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
                    format: Upscaling::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
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

            let depth_texture = Upscaling::create_depth_texture(
                &device,
                camera_width,
                camera_height,
                &texture_bind_group_layout,
            );

            Upscaling {
                texture,
                w2p,
                shader,
                pipeline_layout,
                render_pipeline,
                depth_texture,
            }
        };

        let sprite_ids = Rc::new(RefCell::new(HashSet::new()));

        let quad_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&Self::INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,

            downscaling,
            upscaling,

            quad_index_buffer,

            texture_bind_group_layout,

            camera_width,
            camera_height,

            sprite_ids,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            let vertices = Downscaling::vertices(
                self.size.width,
                self.size.height,
                self.camera_width,
                self.camera_height,
            );
            self.queue.write_buffer(
                &self.downscaling.vertex_buffer,
                0,
                bytemuck::cast_slice(&vertices),
            );
        }
    }

    pub fn render(&mut self, mut dq: DrawQueue) {
        let mut sprites: Vec<&Sprite> = vec![];
        let mut depth = 1;
        let mut instances: HashMap<usize, Vec<Instance>> = HashMap::new();
        for job in dq.0.iter_mut() {
            match job {
                DrawJob::Sprite(sprite, params) => {
                    let instance = Instance {
                        offset: [params.position.x, params.position.y, depth as f32],
                        flip_mask: if params.flip_x && params.flip_y {
                            Instance::FLIP_X | Instance::FLIP_Y
                        } else if params.flip_x {
                            Instance::FLIP_X
                        } else if params.flip_y {
                            Instance::FLIP_Y
                        } else {
                            0
                        },
                    };
                    if instances.contains_key(&sprite.id) {
                        let id = &sprite.id;
                        instances.get_mut(id).unwrap().push(instance);
                    } else {
                        instances.insert(sprite.id, vec![instance]);
                        sprites.push(sprite);
                    }
                }
                DrawJob::SpriteSheet(s, p) => {
                    //sprites.push(&s.sprite);
                }
            }
            depth += 1;
        }
        println!("{:#?}", instances);
        let instance_buffers: HashMap<usize, wgpu::Buffer> = instances
            .iter()
            .map(|(k, instance_vec)| {
                (
                    *k,
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: None,
                            contents: bytemuck::cast_slice(&instance_vec),
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                )
            })
            .collect();

        // start rendering
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.upscaling.texture.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.upscaling.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });
        render_pass.set_pipeline(&self.upscaling.render_pipeline);
        render_pass.set_bind_group(1, &self.upscaling.w2p.bind_group, &[]);

        for sprite in sprites {
            let instance_buffer = instance_buffers.get(&sprite.id).unwrap();
            render_pass.set_bind_group(0, &sprite.texture.bind_group, &[]);
            render_pass.set_vertex_buffer(0, sprite.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
            render_pass
                .set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(
                0..Self::INDICES.len() as u32,
                0,
                0..instances.get(&sprite.id).unwrap().len() as u32,
            );
        }

        drop(render_pass);

        self.queue.submit(iter::once(encoder.finish()));

        // draw downscaled version
        let target = self.surface.get_current_texture().unwrap();
        let view = target
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.downscaling.render_pipeline);
        render_pass.set_bind_group(0, &self.upscaling.texture.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.downscaling.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..Self::INDICES.len() as u32, 0, 0..1);

        drop(render_pass);

        self.queue.submit(iter::once(encoder.finish()));

        target.present();
    }
}

pub struct DrawQueue<'a>(Vec<DrawJob<'a>>);

impl<'a> DrawQueue<'a> {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn draw_sprite(&mut self, sprite: &'a Sprite, params: DrawParams) {
        self.0.push(DrawJob::Sprite(sprite, params));
    }

    pub fn draw_sprite_sheet(&mut self, sprite_sheet: &'a SpriteSheet, params: DrawParams) {
        self.0.push(DrawJob::SpriteSheet(sprite_sheet, params));
    }
}
