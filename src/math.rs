use std::ops::Add;

use glam::UVec2;
use wgpu::Extent3d;

pub struct Rectangle<T> {
    pub x: T,
    pub y: T,
    pub w: T,
    pub h: T,
}
pub type Rect = Rectangle<f32>;

impl<T> Rectangle<T>
where
    T: Add<Output = T> + Copy,
{
    pub fn right(&self) -> T {
        self.x + self.w
    }
    pub fn top(&self) -> T {
        self.y
    }
    pub fn left(&self) -> T {
        self.x
    }
    pub fn bottom(&self) -> T {
        self.y + self.h
    }
}

pub fn extend3d_to_uvec2(e: &Extent3d) -> UVec2 {
    UVec2::new(e.width as u32, e.height as u32)
}
