use std::{cell::RefCell, collections::HashSet, rc::Rc};

use glam::*;

use super::texture::GPUTexture;

#[derive(Debug)]
pub enum Origin {
    Precise(Vec2),
    TopLeft,
    CenterLeft,
    BottomLeft,
    TopMiddle,
    Center,
    BottomMiddle,
    TopRight,
    CenterRight,
    BottomRight,
}

impl Origin {
    pub fn as_vec2(&self, by: Vec2) -> Vec2 {
        match self {
            Origin::TopLeft => Vec2::new(0.0, 0.0) * by,
            Origin::CenterLeft => Vec2::new(0.0, 0.5) * by,
            Origin::BottomLeft => Vec2::new(0.0, 1.0) * by,
            Origin::TopMiddle => Vec2::new(0.5, 0.0) * by,
            Origin::Center => Vec2::new(0.5, 0.5) * by,
            Origin::BottomMiddle => Vec2::new(0.5, 1.0) * by,
            Origin::TopRight => Vec2::new(1.0, 0.0) * by,
            Origin::CenterRight => Vec2::new(1.0, 0.5) * by,
            Origin::BottomRight => Vec2::new(1.0, 1.0) * by,
            Origin::Precise(v) => *v,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    pub offset: [f32; 4],
    pub flip_mask: i32,
    pub ui: i32,
}

impl SpriteInstance {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        5 => Float32x4,
        6 => Sint32,
        7 => Sint32,
    ];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[derive(Debug)]
pub struct Sprite {
    pub vertex_buffer: wgpu::Buffer,
    pub id: usize,
    pub ids: Rc<RefCell<HashSet<usize>>>,
    pub texture: GPUTexture,
    pub origin: Origin,
    pub instances: RefCell<Vec<SpriteInstance>>,
}

impl Drop for Sprite {
    fn drop(&mut self) {
        self.ids.borrow_mut().remove(&self.id);
    }
}

impl Sprite {
    pub fn origin(&self) -> Vec2 {
        self.origin.as_vec2(self.texture.size.as_vec2())
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

    pub fn from_size(width: f32, height: f32) -> [Self; 4] {
        [
            Self {
                position: [0.0, 0.0],
                uv_mask: crate::graphics::TOP_LEFT,
            },
            Self {
                position: [width, 0.0],
                uv_mask: crate::graphics::TOP_RIGHT,
            },
            Self {
                position: [0.0, height],
                uv_mask: crate::graphics::BOTTOM_LEFT,
            },
            Self {
                position: [width, height],
                uv_mask: crate::graphics::BOTTOM_RIGHT,
            },
        ]
    }
}
