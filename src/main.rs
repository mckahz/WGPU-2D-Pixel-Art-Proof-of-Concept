#![allow(dead_code)]
pub mod game;
pub mod player;

fn main() {
    pollster::block_on(cowboy_dot_exe::run());
}
