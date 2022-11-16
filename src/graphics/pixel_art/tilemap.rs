//TODO: god find some other tile editor.
//TODO: add ids to Layers avoid duplication

use std::collections::HashMap;

use glam::Vec2;

use super::{sprite::Sprite, texture::GPUTexture};

/// Describes a tilemap, along with it's tilesets and textures. Can be used to render tiled exports.
pub struct TileMap {
    pub tile_width: u32,
    pub tile_height: u32,
    pub tile_layers: HashMap<String, TileLayer>,
    pub image_layers: HashMap<String, ImageLayer>,
}

pub struct ImageLayer {
    pub position: Vec2,
    pub sprite: Sprite,
    pub repeat_x: bool,
    pub repeat_y: bool,
}

pub struct TileLayer {
    pub name: String,
    pub tiles: Vec<Tile>,
    pub texture: GPUTexture,
    pub vertex_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub instance_count: usize,
    pub tile_depth_buffer: wgpu::Buffer,
    pub tile_depth_bind_group: wgpu::BindGroup,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TileInstance {
    pub offset: [f32; 2],
    pub flip_mask: i32,
    pub tile_index: i32,
}

impl TileInstance {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![5 => Float32x2, 6 => Sint32, 7 => Sint32];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv_mask: i32,
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Sint32];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub struct Tile {
    pub flip_x: bool,
    pub flip_y: bool,
    pub id: u32,
    pub x: i32,
    pub y: i32,
}
