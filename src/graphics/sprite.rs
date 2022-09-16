use std::{cell::RefCell, collections::HashSet, rc::Rc};

use crate::graphics::texture::*;
use glam::*;

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

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// TODO: Make the flipping of the sprite somehow occur on the gpu
pub struct Instance {
    pub offset: [f32; 4],
}

impl Instance {
    pub const FLIP_X: u32 = 0b10;
    pub const FLIP_Y: u32 = 0b01;

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
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
}

impl Drop for Sprite {
    fn drop(&mut self) {
        self.ids.borrow_mut().remove(&self.id);
    }
}

impl Sprite {
    pub fn origin(&self) -> Vec2 {
        let size = self.texture.size.as_vec2();
        match self.origin {
            Origin::TopLeft => Vec2::new(0.0, 0.0) * size,
            Origin::CenterLeft => Vec2::new(0.0, 0.5) * size,
            Origin::BottomLeft => Vec2::new(0.0, 1.0) * size,
            Origin::TopMiddle => Vec2::new(0.5, 0.0) * size,
            Origin::Center => Vec2::new(0.5, 0.5) * size,
            Origin::BottomMiddle => Vec2::new(0.5, 1.0) * size,
            Origin::TopRight => Vec2::new(1.0, 0.0) * size,
            Origin::CenterRight => Vec2::new(1.0, 0.5) * size,
            Origin::BottomRight => Vec2::new(1.0, 1.0) * size,
            Origin::Precise(v) => v,
        }
    }
}

pub enum Orientation {
    Vertical,
    Horizontal,
}

pub enum FrameRate {
    Constant(f32),
    Variable(Vec<f32>),
}

pub struct SpriteSheet {
    pub sprite: Sprite,
    pub count: u8,
    pub t: f32,
    pub frame_rate: FrameRate,
    pub orientation: Orientation,
}

impl SpriteSheet {
    pub fn frame(&self) -> usize {
        self.t.floor() as usize
    }

    pub fn advance(&mut self, delta: f32) {
        self.t += delta
            / match &self.frame_rate {
                FrameRate::Constant(spf) => spf,
                FrameRate::Variable(spfs) => spfs.get(self.frame()).unwrap(),
            };
        if self.t > self.count as f32 {
            self.t -= self.t;
        }
    }

    pub fn size(&self) -> UVec2 {
        match self.orientation {
            Orientation::Vertical => UVec2::new(
                self.sprite.texture.size.x,
                self.sprite.texture.size.y / (self.count as u32),
            ),
            Orientation::Horizontal => UVec2::new(
                self.sprite.texture.size.x / (self.count as u32),
                self.sprite.texture.size.y,
            ),
        }
    }
}

pub enum SpriteType {
    Sprite,
    SpriteSheet,
}
