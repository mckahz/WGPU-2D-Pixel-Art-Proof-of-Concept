use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

use self::vertex::Vertex;

mod vertex;

pub struct Downscale {
    pub vertex_buffer: wgpu::Buffer,
    pub shader: wgpu::ShaderModule,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub render_pipeline: wgpu::RenderPipeline,
}

impl Downscale {
    pub fn new(
        device: &wgpu::Device,
        camera_width: u32,
        camera_height: u32,
        config: &wgpu::SurfaceConfiguration,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        size: PhysicalSize<u32>,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("downscale.wgsl").into()),
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

        let vertices = Self::vertices(size.width, size.height, camera_width, camera_height);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
        });

        Self {
            vertex_buffer,
            shader,
            pipeline_layout,
            render_pipeline,
        }
    }

    pub fn vertices(
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
                uv: [0.0, 0.0],
            },
            Vertex {
                position: [half_width, half_height, 0.0],
                uv: [1.0, 0.0],
            },
            Vertex {
                position: [-half_width, -half_height, 0.0],
                uv: [0.0, 1.0],
            },
            Vertex {
                position: [half_width, -half_height, 0.0],
                uv: [1.0, 1.0],
            },
        ]
    }
}
