use cowboy_dot_exe::run;

fn main() {
    pollster::block_on(run());
}
