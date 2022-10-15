use std::ops::{Add, Sub};

use glam::{UVec2, Vec2};
use wgpu::Extent3d;

#[derive(Debug)]
pub struct Rectangle<T> {
    pub x: T,
    pub y: T,
    pub w: T,
    pub h: T,
}
pub type Rect = Rectangle<f32>;

impl<T> Rectangle<T>
where
    T: Add<Output = T> + Copy + Sub<Output = T> + PartialOrd,
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

    pub fn from_pos(x1: T, y1: T, x2: T, y2: T) -> Self {
        Self {
            x: x1,
            y: y1,
            w: x2 - x1,
            h: y2 - y1,
        }
    }

    pub fn contains(&self, other: &Self) -> bool {
        !(self.right() < other.left()
            || self.left() > other.right()
            || self.top() > other.bottom()
            || self.bottom() < other.top())
    }
}

impl Rect {
    pub fn translate(&self, other: Vec2) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            ..*self
        }
    }
}

pub fn extend3d_to_uvec2(e: &Extent3d) -> UVec2 {
    UVec2::new(e.width as u32, e.height as u32)
}
