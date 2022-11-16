use std::{cell::RefCell, collections::HashSet, rc::Rc};

use glam::*;

use super::{sprite::Origin, texture::GPUTexture};

#[derive(Debug)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

impl Orientation {
    pub fn as_int(&self) -> i32 {
        match self {
            Orientation::Vertical => 0,
            Orientation::Horizontal => 1,
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum FrameRate {
    Constant(f32),
    Variable(Vec<f32>),
    None,
}

#[derive(Debug)]
pub struct SpriteSheet {
    pub id: usize,
    pub ids: Rc<RefCell<HashSet<usize>>>,
    pub instances: RefCell<Vec<SpriteSheetInstance>>,
    pub vertex_buffer: wgpu::Buffer,
    pub texture: GPUTexture,
    pub origin: Origin,
    pub count: u8,
    pub frame_rate: FrameRate,
    pub orientation: Orientation,
    pub data_buffer: wgpu::Buffer,
    pub data_bind_group: wgpu::BindGroup,
}

impl SpriteSheet {
    pub fn get_vertex_info(&self) -> (f32, f32, f32, f32, f32, f32) {
        let (tw, th) = self.texture.size.as_vec2().into();
        let frame = 0 as f32;
        let count = self.count as f32;
        let (w, h, left, right, top, bottom);
        match self.orientation {
            Orientation::Vertical => {
                w = tw;
                h = th / count;
                left = 0.0;
                right = 1.0;
                top = frame / count;
                bottom = (frame + 1.0) / count;
            }
            Orientation::Horizontal => {
                w = tw / count;
                h = th;
                left = frame / count;
                right = (frame + 1.0) / count;
                top = 0.0;
                bottom = 1.0;
            }
        }
        (w, h, left, right, top, bottom)
    }

    pub fn size(&self) -> UVec2 {
        match self.orientation {
            Orientation::Vertical => UVec2::new(
                self.texture.size.x,
                self.texture.size.y / (self.count as u32),
            ),
            Orientation::Horizontal => UVec2::new(
                self.texture.size.x / (self.count as u32),
                self.texture.size.y,
            ),
        }
    }

    pub fn origin(&self) -> Vec2 {
        self.origin.as_vec2(self.size().as_vec2())
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteSheetInstance {
    pub offset: [f32; 4],
    pub flip_mask: i32,
    pub t: i32,
}

impl SpriteSheetInstance {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![5 => Float32x4, 6 => Sint32, 7 => Sint32];

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
pub struct SpriteSheetData {
    pub count: i32,
    pub horizontal: i32,
}

impl SpriteSheetData {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![8 => Sint32, 9 => Sint32];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub struct AnimationTime {
    t: f32,
    frame: u8,
    current_frame_duration: f32,
    frame_rate: FrameRate,
    max: u8,
}

impl AnimationTime {
    pub fn current_frame_duration(&self) -> f32 {
        match &self.frame_rate {
            FrameRate::Constant(frame_duration) => *frame_duration,
            FrameRate::Variable(frame_durations) => frame_durations[self.frame as usize],
            FrameRate::None => 0.0,
        }
    }

    pub fn new(sprite_sheet: &SpriteSheet) -> Self {
        let mut inst = Self {
            t: 0.0,
            frame: 0,
            frame_rate: sprite_sheet.frame_rate.clone(),
            current_frame_duration: 0.0,
            max: sprite_sheet.count,
        };
        inst.current_frame_duration = inst.current_frame_duration();
        inst
    }

    //TODO: make this return u8, or make SpriteSheet.count a u32
    pub fn frame(&self) -> u32 {
        self.frame as u32
    }

    pub fn advance(&mut self, delta: f32) {
        self.t += delta;

        let cfd = self.current_frame_duration();
        if self.t > cfd {
            self.t %= cfd;
            self.frame = (self.frame + 1) % self.max;
        }
    }
}
