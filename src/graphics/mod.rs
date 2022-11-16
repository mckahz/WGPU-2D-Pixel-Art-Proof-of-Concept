pub mod downscale;
pub mod pixel_art;

/*  TODO:
T   collision
T   camera
T   tilemap import
T   make flipping instance based
T   make animation frame instance based
F   wasm
F   alternating attacks
F   palette swapping
F   hold after attack
F   hitboxes
F   hurtboxes
F   player damaging
F   slow down when attacking
F   refactor player state system to also include attacks?
F   refactor attacking things out of Player
F   player's double jump is a burst jump
F   dashing
F   allow specifying the size/rect of the sprite
F   maybe add resources handles?
*/
use itertools::iproduct;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    iter,
    rc::Rc,
};
use wgpu::{util::DeviceExt, BindGroupEntry};

use crate::{
    file_system::{self, LoadError},
    graphics::pixel_art::{
        sprite::*,
        sprite_sheet::*,
        texture::*,
        tilemap::{ImageLayer, Tile},
    },
    math::extend3d_to_uvec2,
};
use glam::*;
use winit::{dpi::PhysicalSize, window::Window};

use self::{
    downscale::Downscale,
    pixel_art::{
        sprite_sheet::SpriteSheet,
        tilemap::{TileInstance, TileLayer, TileMap},
        Camera, PixelArt,
    },
};

pub const TOP_LEFT: i32 = 0b1;
pub const TOP_RIGHT: i32 = 0b10;
pub const BOTTOM_LEFT: i32 = 0b100;
pub const BOTTOM_RIGHT: i32 = 0b1000;
pub const FLIP_X: i32 = 0b01;
pub const FLIP_Y: i32 = 0b10;

#[derive(Debug, Clone, Copy)]
pub struct DrawParams {
    pub position: Vec2,
    pub flip_x: bool,
    pub flip_y: bool,
    pub camera_locked: bool,
}

pub fn new_flip_mask(flip_x: bool, flip_y: bool) -> i32 {
    if flip_x && flip_y {
        FLIP_X | FLIP_Y
    } else if flip_x {
        FLIP_X
    } else if flip_y {
        FLIP_Y
    } else {
        0
    }
}

impl DrawParams {
    fn to_flip_mask(&self) -> i32 {
        new_flip_mask(self.flip_x, self.flip_y)
    }

    pub fn ui(mut self, enabled: bool) -> Self {
        self.camera_locked = enabled;
        self
    }
}

pub enum DrawJob<'a> {
    Sprite(&'a Sprite, DrawParams),
    SpriteSheet(&'a SpriteSheet, u32, DrawParams),
    TileLayer(&'a TileLayer),
}

pub struct DrawQueue<'a>(pub Vec<DrawJob<'a>>);

pub struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,

    downscaling: Downscale,
    pixel_art: PixelArt,

    camera_width: u32,
    camera_height: u32,

    texture_bind_group_layout: wgpu::BindGroupLayout,
    sprite_ids: Rc<RefCell<HashSet<usize>>>,

    quad_index_buffer: wgpu::Buffer,
}

impl Default for DrawParams {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            flip_x: false,
            flip_y: false,
            camera_locked: false,
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

impl Renderer {
    pub fn update_camera(&mut self, offset: Vec2) {
        self.pixel_art.camera = Camera::from(offset);
        self.queue.write_buffer(
            &self.pixel_art.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.pixel_art.camera]),
        )
    }

    pub const INDICES: [u16; 6] = [2, 1, 0, 1, 2, 3];

    pub fn load_tilemap(&self, path: &str) -> Result<TileMap, LoadError> {
        use crate::graphics::pixel_art::tilemap::Vertex;

        let path = &file_system::to_asset_path(path);
        let mut loader = tiled::Loader::new();
        let tmx = match loader.load_tmx_map(path) {
            Ok(tile_map) => tile_map,
            Err(_) => return Err(LoadError::PathNotFound(path.to_owned())),
        };

        let mut tile_layers = HashMap::new();
        let mut image_layers = HashMap::new();

        for layer in tmx.layers() {
            match layer.layer_type() {
                tiled::LayerType::TileLayer(tile_layer) => {
                    let name = layer.name.clone();
                    let tiles: Vec<Tile> = iproduct!(
                        0..tile_layer.width().unwrap(),
                        0..tile_layer.height().unwrap()
                    )
                    .filter_map(|(x, y)| {
                        tile_layer.get_tile(x as i32, y as i32).map(|t| Tile {
                            flip_x: t.flip_h,
                            flip_y: t.flip_v,
                            id: t.id(),
                            x: x as i32,
                            y: y as i32,
                        })
                    })
                    .collect();

                    let coords = iproduct!(
                        0..tile_layer.width().unwrap(),
                        0..tile_layer.height().unwrap()
                    );

                    //get first used tile
                    //TODO: find some way to short circuit this
                    let (x, y) = coords
                        .skip_while(|(x, y)| tile_layer.get_tile(*x as i32, *y as i32).is_none())
                        .nth(0)
                        .unwrap();

                    let texture = tile_layer
                        .get_tile(x as i32, y as i32)
                        .as_ref()
                        .unwrap()
                        .get_tileset()
                        .image
                        .as_ref()
                        .unwrap()
                        .source
                        .to_str()
                        .unwrap()["./assets".len()..]
                        .to_owned();

                    let mut instances: Vec<TileInstance> = vec![];
                    let mut instance_count = 0;

                    for tile in tiles.iter() {
                        instances.push(TileInstance {
                            offset: [
                                (tmx.tile_width as i32 * tile.x) as f32,
                                (tmx.tile_height as i32 * tile.y) as f32,
                            ],
                            flip_mask: new_flip_mask(tile.flip_x, tile.flip_y),
                            tile_index: tile.id as i32,
                        });
                        instance_count += 1;
                    }

                    let instance_buffer =
                        self.device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: None,
                                contents: bytemuck::cast_slice(&instances),
                                usage: wgpu::BufferUsages::VERTEX,
                            });

                    let vertex_buffer =
                        self.device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: None,
                                contents: bytemuck::cast_slice(&[
                                    Vertex {
                                        position: [0.0, 0.0],
                                        uv_mask: TOP_LEFT,
                                    },
                                    Vertex {
                                        position: [tmx.tile_width as f32, 0.0],
                                        uv_mask: TOP_RIGHT,
                                    },
                                    Vertex {
                                        position: [0.0, tmx.tile_height as f32],
                                        uv_mask: BOTTOM_LEFT,
                                    },
                                    Vertex {
                                        position: [tmx.tile_width as f32, tmx.tile_height as f32],
                                        uv_mask: BOTTOM_RIGHT,
                                    },
                                ]),
                                usage: wgpu::BufferUsages::VERTEX,
                            });

                    let tile_depth_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: None,
                        size: std::mem::size_of::<f32>() as u64,
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                        mapped_at_creation: false,
                    });
                    let tile_depth_bind_group =
                        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: None,
                            layout: &self.pixel_art.tile_depth_bind_group_layout,
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: tile_depth_buffer.as_entire_binding(),
                            }],
                        });

                    tile_layers.insert(
                        name.clone(),
                        TileLayer {
                            name: name.clone(),
                            texture: self.load_texture(&texture).unwrap(),
                            vertex_buffer,
                            instance_buffer,
                            instance_count,
                            tile_depth_buffer,
                            tile_depth_bind_group,
                            tiles,
                        },
                    );
                }
                tiled::LayerType::ImageLayer(image_layer) => {
                    let sprite = self.load_sprite(
                        Origin::TopLeft,
                        &image_layer.image.as_ref().unwrap().source.to_str().unwrap()
                            ["./assets".len()..],
                    )?;

                    image_layers.insert(
                        layer.name.clone(),
                        ImageLayer {
                            sprite,
                            repeat_x: match layer.properties.get("Repeat X") {
                                Some(tiled::PropertyValue::BoolValue(b)) => *b,
                                _ => false,
                            },
                            repeat_y: match layer.properties.get("Repeat Y") {
                                Some(tiled::PropertyValue::BoolValue(b)) => *b,
                                _ => false,
                            },
                            position: Vec2::new(layer.offset_x, layer.offset_y),
                        },
                    );
                }
                _ => {}
            }
        }

        Ok(TileMap {
            tile_layers,
            image_layers,
            tile_width: tmx.tile_width,
            tile_height: tmx.tile_height,
        })
    }

    pub fn load_texture(&self, path: &str) -> Result<GPUTexture, LoadError> {
        let img = file_system::load_image(path)?;
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
        let sampler = self
            .device
            .create_sampler(&wgpu::SamplerDescriptor::default());

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

        Ok(GPUTexture {
            bind_group,
            texture,
            view,
            size: extend3d_to_uvec2(&size),
            sampler,
        })
    }

    pub fn load_sprite(&self, origin: Origin, path: &str) -> Result<Sprite, LoadError> {
        let texture = self.load_texture(path)?;

        let (width, height) = texture.size.as_vec2().into();

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&pixel_art::sprite::Vertex::from_size(
                    width, height,
                )),
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

        Ok(Sprite {
            vertex_buffer,
            texture,
            origin,
            id,
            ids,
            instances: RefCell::new(vec![]),
        })
    }

    pub fn load_sprite_sheet(
        &self,
        origin: Origin,
        path: &str,
        count: u8,
        frame_rate: FrameRate,
        orientation: Orientation,
    ) -> Result<SpriteSheet, LoadError> {
        let texture = self.load_texture(path)?;
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

        use pixel_art::sprite_sheet::Vertex;
        let (width, height) = (texture.size.as_vec2()
            / match orientation {
                Orientation::Vertical => Vec2::new(1.0, count as f32),
                Orientation::Horizontal => Vec2::new(count as f32, 1.0),
            })
        .into();
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[
                    Vertex {
                        position: [0.0, 0.0],
                        uv_mask: TOP_LEFT,
                    },
                    Vertex {
                        position: [width, 0.0],
                        uv_mask: TOP_RIGHT,
                    },
                    Vertex {
                        position: [0.0, height],
                        uv_mask: BOTTOM_LEFT,
                    },
                    Vertex {
                        position: [width, height],
                        uv_mask: BOTTOM_RIGHT,
                    },
                ]),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let data_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[SpriteSheetData {
                    count: count as i32,
                    horizontal: orientation.as_int(),
                }]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::VERTEX,
            });
        let data_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.pixel_art.sprite_sheet_data_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: data_buffer.as_entire_binding(),
            }],
        });

        Ok(SpriteSheet {
            count,
            frame_rate,
            orientation,
            id,
            ids,
            texture,
            origin,
            vertex_buffer,
            instances: RefCell::new(vec![]),
            data_buffer,
            data_bind_group,
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
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
            };
            surface.configure(&device, &config);

            (instance, surface, adapter, device, queue, config)
        };

        // ------------------------------------------------------------------------------------------- Texture Bind Group
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
        let downscaling = Downscale::new(
            &device,
            camera_width,
            camera_height,
            &config,
            &texture_bind_group_layout,
            size,
        );

        // ------------------------------------------------------------------------------------------- Upscaling
        let pixel_art = PixelArt::new(
            &device,
            camera_width,
            camera_height,
            &config,
            &texture_bind_group_layout,
        );

        // ------------------------------------------------------------------------------------------- Sprites
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
            pixel_art,

            quad_index_buffer,

            texture_bind_group_layout,
            sprite_ids: Rc::new(RefCell::new(HashSet::new())),

            camera_width,
            camera_height,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.pixel_art.depth_texture = TextureRaw::create_depth_texture(
                &self.device,
                self.camera_width,
                self.camera_height,
            );

            let vertices = Downscale::vertices(
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

    fn flatten_draw_queue<'a>(
        &self,
        mut dq: DrawQueue<'a>,
    ) -> (Vec<&'a Sprite>, Vec<&'a SpriteSheet>, Vec<&'a TileLayer>) {
        let mut sprites: Vec<&Sprite> = vec![];
        let mut sprite_ids = HashSet::new();
        let mut sprite_sheets: Vec<&SpriteSheet> = vec![];
        let mut sprite_sheet_ids = HashSet::new();
        let mut tile_layers: Vec<&TileLayer> = vec![];
        let mut depth = 1.0;
        let depth_step = 1.0 / (dq.0.len() as f32);
        for job in dq.0.iter_mut() {
            match job {
                DrawJob::Sprite(sprite, params) => {
                    let origin = sprite.origin();
                    let size = sprite.texture.size.as_vec2();

                    // flip logic
                    let flipped_offset = params.position - size + origin;
                    let default_offset = params.position - origin;
                    let offset = if params.flip_x && params.flip_y {
                        flipped_offset
                    } else if params.flip_x {
                        Vec2::new(flipped_offset.x, default_offset.y)
                    } else if params.flip_y {
                        Vec2::new(default_offset.x, flipped_offset.y)
                    } else {
                        default_offset
                    };
                    let instance = SpriteInstance {
                        offset: [offset.x, offset.y, depth, 0.0],
                        flip_mask: params.to_flip_mask(),
                        ui: params.camera_locked as i32,
                    };

                    sprite.instances.borrow_mut().push(instance);
                    if !sprite_ids.contains(&sprite.id) {
                        sprites.push(sprite);
                        sprite_ids.insert(sprite.id);
                    }
                }
                DrawJob::SpriteSheet(sprite_sheet, t, params) => {
                    let origin = sprite_sheet.origin();
                    let size = sprite_sheet.size().as_vec2();

                    let fo = params.position - size + origin;
                    let defo = params.position - origin;
                    let offset = if params.flip_x && params.flip_y {
                        fo
                    } else if params.flip_x {
                        Vec2::new(fo.x, defo.y)
                    } else if params.flip_y {
                        Vec2::new(defo.x, fo.y)
                    } else {
                        defo
                    };

                    let instance = SpriteSheetInstance {
                        offset: [offset.x, offset.y, depth, 0.0],
                        flip_mask: new_flip_mask(params.flip_x, params.flip_y),
                        t: *t as i32,
                    };

                    sprite_sheet.instances.borrow_mut().push(instance);
                    if !sprite_sheet_ids.contains(&sprite_sheet.id) {
                        sprite_sheets.push(sprite_sheet);
                        sprite_sheet_ids.insert(sprite_sheet.id);
                    }
                }
                DrawJob::TileLayer(tile_layer) => {
                    self.queue.write_buffer(
                        &tile_layer.tile_depth_buffer,
                        0,
                        bytemuck::cast_slice(&[depth]),
                    );
                    tile_layers.push(tile_layer);
                }
            }

            depth -= depth_step;
        }

        (sprites, sprite_sheets, tile_layers)
    }

    pub fn render(&mut self, dq: DrawQueue) {
        let (sprites, sprite_sheets, tile_layers) = self.flatten_draw_queue(dq);

        // Setup rendering
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let sprite_iter: Vec<(&Sprite, wgpu::Buffer)> = sprites
            .into_iter()
            .map(|spr| {
                (
                    spr,
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: None,
                            contents: bytemuck::cast_slice(&spr.instances.borrow()),
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                )
            })
            .collect();

        let sprite_sheet_iter: Vec<(&SpriteSheet, wgpu::Buffer)> = sprite_sheets
            .into_iter()
            .map(|spr| {
                (
                    spr,
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: None,
                            contents: bytemuck::cast_slice(&spr.instances.borrow()),
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                )
            })
            .collect();

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.pixel_art.texture.view,
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
                view: &self.pixel_art.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_bind_group(1, &self.pixel_art.w2p.bind_group, &[]);
        render_pass.set_bind_group(2, &self.pixel_art.camera_bind_group, &[]);

        // Render Sprites
        render_pass.set_pipeline(&self.pixel_art.sprite_render_pipeline);

        for (sprite, instance_buffer) in sprite_iter.iter() {
            render_pass.set_bind_group(0, &sprite.texture.bind_group, &[]);
            render_pass.set_vertex_buffer(0, sprite.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
            render_pass
                .set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(
                0..Self::INDICES.len() as u32,
                0,
                0..sprite.instances.borrow().len() as u32,
            );

            sprite.instances.borrow_mut().clear();
        }

        // Render Tile Layers
        render_pass.set_pipeline(&self.pixel_art.tile_layer_render_pipeline);

        for tile_layer in tile_layers.iter() {
            render_pass.set_bind_group(3, &tile_layer.tile_depth_bind_group, &[]);
            render_pass.set_bind_group(0, &tile_layer.texture.bind_group, &[]);
            render_pass.set_vertex_buffer(0, tile_layer.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, tile_layer.instance_buffer.slice(..));
            render_pass
                .set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(
                0..Self::INDICES.len() as u32,
                0,
                0..tile_layer.instance_count as u32,
            );
        }

        // Render Sprite Sheets
        render_pass.set_pipeline(&self.pixel_art.sprite_sheet_render_pipeline);

        for (sprite_sheet, instance_buffer) in sprite_sheet_iter.iter() {
            render_pass.set_bind_group(3, &sprite_sheet.data_bind_group, &[]);
            render_pass.set_bind_group(0, &sprite_sheet.texture.bind_group, &[]);
            render_pass
                .set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_vertex_buffer(0, sprite_sheet.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
            render_pass.draw_indexed(
                0..Self::INDICES.len() as u32,
                0,
                0..sprite_sheet.instances.borrow().len() as u32,
            );
            sprite_sheet.instances.borrow_mut().clear();
        }

        drop(render_pass);

        // Render To Screen
        let screen = self.surface.get_current_texture().unwrap();
        let view = screen
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

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
        render_pass.set_bind_group(0, &self.pixel_art.texture.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.downscaling.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..Self::INDICES.len() as u32, 0, 0..1);

        drop(render_pass);

        self.queue.submit(iter::once(encoder.finish()));

        screen.present();
    }
}

impl<'a> DrawQueue<'a> {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn append(&mut self, mut dq: DrawQueue<'a>) {
        self.0.append(&mut dq.0);
    }

    pub fn sprite(&mut self, sprite: &'a Sprite, params: DrawParams) {
        self.0.push(DrawJob::Sprite(sprite, params));
    }

    pub fn sheet(&mut self, sprite_sheet: &'a SpriteSheet, t: u32, params: DrawParams) {
        self.0.push(DrawJob::SpriteSheet(sprite_sheet, t, params));
    }

    /// Tile layers may only be drawn once. Future draw calls on the same tile layer will be ignored.
    pub fn tile_layer(&mut self, tile_map: &'a TileMap, layer: &str) {
        self.0
            .push(DrawJob::TileLayer(tile_map.tile_layers.get(layer).unwrap()));
    }

    /// Tile layers may only be drawn once. Future draw calls on the same tile layer will be ignored.
    pub fn tile_image(&mut self, tile_map: &'a TileMap, layer: &str) {
        let image_layer = tile_map.image_layers.get(layer).unwrap();
        let mut draw_spr = |offset| {
            self.0.push(DrawJob::Sprite(
                &image_layer.sprite,
                DrawParams::from_pos(offset + image_layer.position),
            ))
        };
        //TODO: make tile as long as necessary
        let x_lim = 4;
        let y_lim = 2;
        if image_layer.repeat_x && image_layer.repeat_y {
            for i in 0..x_lim {
                for j in 0..y_lim {
                    draw_spr(Vec2::new(
                        (i * image_layer.sprite.texture.size.x) as f32,
                        (j * image_layer.sprite.texture.size.y) as f32,
                    ));
                }
            }
        } else if image_layer.repeat_x {
            for i in 0..x_lim {
                draw_spr(Vec2::new(
                    (i * image_layer.sprite.texture.size.x) as f32,
                    0.0,
                ));
            }
        } else if image_layer.repeat_y {
            for i in 0..y_lim {
                draw_spr(Vec2::new(
                    0.0,
                    (i * image_layer.sprite.texture.size.y) as f32,
                ));
            }
        } else {
            draw_spr(Vec2::ZERO);
        }
    }
}
