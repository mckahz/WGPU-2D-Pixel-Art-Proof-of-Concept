#![allow(dead_code)]

fn main() {
    pollster::block_on(cowboy_dot_exe::run());
}
