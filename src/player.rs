use glam::Vec2;

use crate::{graphics::sprite::*, input::Input, math::Rect};

pub struct Sprites {
    pub idle: Sprite,
    pub jump_fall: Sprite,
    pub jump_land: Sprite,
    pub jump_rise: Sprite,
    pub run_start: Sprite,
}

pub struct SpriteSheets {
    pub run: SpriteSheet,
}

pub enum JumpState {
    Rise,
    Fall,
    Land,
}

pub enum RunState {
    Start,
    Sprint,
}

pub enum State {
    Idle,
    Jumping(JumpState),
    Run(RunState),
}

pub struct Player {
    pub state: State,
    pub sprites: Sprites,
    pub sprite_sheets: SpriteSheets,
    pub data: Data,
}

const GROUND_HEIGHT: f32 = 208 as f32;

pub const H_SPEED: f32 = 200.0;
pub const GRAVITY: f32 = 2000.0;
pub const MAX_JUMPS: i8 = 2;
pub const JUMP_SPEED: f32 = 500.0;

impl Player {
    pub fn update(&mut self, delta: f32, input: &Input) {
        let left_pressed = input.left.pressed;
        let right_pressed = input.right.pressed;
        let direction = if !left_pressed && right_pressed {
            1
        } else if left_pressed && !right_pressed {
            -1
        } else {
            0
        } as f32;
        if direction == 0.0 {
            self.data.velocity.x *= 0.3;
        } else if direction.signum() == self.data.velocity.x.signum() {
            self.data.velocity.x += (direction * H_SPEED - self.data.velocity.x) * 0.5;
        } else {
            self.data.velocity.x += (direction * H_SPEED - self.data.velocity.x) * 0.7;
        }

        // update the player
        match &mut self.state {
            State::Idle => {
                self.data.v_collide();
                self.data.jump_test(input);
            }
            State::Jumping(_) => {
                self.data.apply_gravity(delta);

                self.data.jump_test(input);

                self.data.update_pos(delta);

                self.data.h_collide();
                self.data.v_collide();
            }
            State::Run(_) => {
                self.data.apply_gravity(delta);

                self.data.update_pos(delta);

                self.data.v_collide();

                self.data.jump_test(input);

                self.data.h_collide();
            }
        }

        // Check for state changes
        loop {
            match &mut self.state {
                State::Idle => {
                    if !self.data.on_ground {
                        self.state = State::Jumping(JumpState::Fall);
                    } else if input.jump.just_pressed() {
                        self.state = State::Jumping(JumpState::Rise);
                    } else if direction != 0.0 {
                        self.state = State::Run(RunState::Start);
                    }
                    break;
                }
                State::Jumping(ref mut jump_state) => {
                    if self.data.on_ground {
                        self.state = State::Idle;
                    } else if self.data.velocity.y > 0.0 {
                        *jump_state = JumpState::Fall;
                    } else if self.data.velocity.y < 0.0 {
                        *jump_state = JumpState::Rise;
                    }
                    break;
                }
                State::Run(ref mut run_state) => {
                    if !self.data.on_ground {
                        self.state = State::Jumping(JumpState::Rise);
                    } else {
                        match run_state {
                            RunState::Start => {
                                if self.data.velocity.x.abs() >= H_SPEED * 0.5 {
                                    *run_state = RunState::Sprint;
                                }
                            }
                            RunState::Sprint => {
                                if self.data.velocity.x.abs() < 0.3 {
                                    self.state = State::Idle;
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }

        self.data.flipped = if self.data.velocity.x > 0.0 {
            false
        } else if self.data.velocity.x < 0.0 {
            true
        } else {
            self.data.flipped
        };

        // advance sprite sheet frames
        match &mut self.state {
            State::Run(state) => match state {
                RunState::Sprint => {
                    self.sprite_sheets.run.advance(delta);
                }
                _ => {}
            },
            _ => {}
        }
    }
}

pub struct Data {
    pub health: i8,
    pub collision_rect: Rect,
    pub velocity: Vec2,
    pub position: Vec2,
    pub jump_count: i8,
    pub on_ground: bool,
    pub flipped: bool,
}

impl Data {
    pub fn apply_gravity(&mut self, delta: f32) {
        self.on_ground = false;
        self.velocity.y += GRAVITY * delta;
    }
    pub fn jump_test(&mut self, input: &Input) {
        if self.jump_count > 0 && input.jump.just_pressed() {
            self.velocity.y = -JUMP_SPEED;
            self.jump_count -= 1;
            self.on_ground = false;
        }
    }
    pub fn update_pos(&mut self, delta: f32) {
        self.position += self.velocity * delta;
    }
    pub fn v_collide(&mut self) {
        if self.position.y > GROUND_HEIGHT {
            self.position.y = GROUND_HEIGHT;
            self.velocity.y = 0.0;
            self.jump_count = MAX_JUMPS;
            self.on_ground = true;
        }
    }
    pub fn h_collide(&mut self) {
        let padding = 4.0;
        let left_bound = padding;
        let right_bound = (super::game::CAMERA_WIDTH as f32) - padding;
        // horizontal
        let left = self.position.x + self.collision_rect.left();
        let right = self.position.x + self.collision_rect.right();
        if left < left_bound {
            self.position.x = left_bound - self.collision_rect.left();
        } else if right > right_bound {
            self.position.x = right_bound - self.collision_rect.right();
        }
    }
}
