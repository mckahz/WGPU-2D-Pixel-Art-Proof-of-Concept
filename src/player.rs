#![allow(dead_code)]

use glam::Vec2;

use pyxl::{
    file_system::LoadError,
    graphics::{
        pixel_art::{sprite::*, sprite_sheet::*},
        DrawParams, DrawQueue, Renderer,
    },
    input::*,
    math::{Rect, Rectangle},
};

use crate::game::LevelGeometry;

pub struct Sprites {
    pub idle: Sprite,
    pub jump_fall: Sprite,
    pub jump_land: Sprite,
    pub jump_rise: Sprite,
    pub run_start: Sprite,
    pub climb: Sprite,
    pub climb_end: Sprite,
    pub run: SpriteSheet,
    pub attack_run_down_poncho: SpriteSheet,
    pub attack_run_down_sword: SpriteSheet,
    pub attack_run_feet: SpriteSheet,
    pub attack_run_up_poncho: SpriteSheet,
    pub attack_run_up_sword: SpriteSheet,
    pub attack_stand_down: SpriteSheet,
    pub attack_stand_up: SpriteSheet,
    pub attack_climb_down: SpriteSheet,
    pub attack_climb_up: SpriteSheet,
    pub slash: SpriteSheet,
}

impl Sprites {
    pub fn update_attack_animations(&mut self, attack_state: &AttackState) {
        let frame = 1.0 - (attack_state.time / ATTACK_DURATION);
        for anim in [
            &mut self.attack_run_down_poncho,
            &mut self.attack_run_down_sword,
            &mut self.attack_run_up_poncho,
            &mut self.attack_run_up_sword,
            &mut self.attack_stand_down,
            &mut self.attack_stand_up,
            &mut self.attack_climb_down,
            &mut self.attack_climb_up,
            &mut self.slash,
        ] {
            //anim.scrub_norm(frame);
        }
        //self.attack_run_feet.scrub(self.run.t);
    }
}

#[derive(Debug)]
pub enum JumpState {
    Rise,
    Fall,
}

#[derive(Debug)]
pub enum RunState {
    Start,
    Sprint,
}

#[derive(Debug)]
pub enum IdleState {
    Landing(f32),
    Crouching,
    Standing,
}

#[derive(Debug)]
pub struct AttackState {
    pub time: f32,
    pub stage: AttackStage,
}

#[derive(Debug)]
pub enum State {
    Idle(IdleState),
    Jump(JumpState),
    Run(RunState),
    AttackIdle(AttackState),
    AttackRun(AttackState),
    AttackJump(AttackState),
}

impl AttackState {
    pub fn advance(&mut self, delta: f32) {
        self.time -= delta;
        if self.time < 0.0 {
            self.stage = match self.stage {
                AttackStage::Up => AttackStage::Finish,
                _ => AttackStage::Finish,
            };
            self.time = ATTACK_DURATION;
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum AttackStage {
    Up,
    Down,
    Finish,
}

pub struct Player {
    pub state: State,
    pub sprites: Sprites,
    pub data: Data,
}

impl Player {
    pub fn new(r: &mut Renderer) -> Result<Self, LoadError> {
        fn player_origin() -> Origin {
            Origin::Precise(Vec2::new(31.0, 22.0))
        }
        let sprites = Sprites {
            idle: r.load_sprite(player_origin(), "player/idle.png")?,
            jump_fall: r.load_sprite(player_origin(), "player/jump_fall.png")?,
            jump_land: r.load_sprite(player_origin(), "player/jump_land.png")?,
            jump_rise: r.load_sprite(player_origin(), "player/jump_rise.png")?,
            run_start: r.load_sprite(player_origin(), "player/run_start.png")?,
            climb: r.load_sprite(player_origin(), "player/climb.png")?,
            climb_end: r.load_sprite(player_origin(), "player/climb_end.png")?,
            run: r.load_sprite_sheet(
                Origin::Precise(Vec2::new(31.0, 23.0)),
                "player/run.png",
                6,
                FrameRate::Constant(0.1),
                Orientation::Horizontal,
            )?,
            attack_run_down_poncho: r.load_sprite_sheet(
                Origin::Precise(Vec2::new(31.0, 31.0)),
                "player/attack_run_down_poncho.png",
                3,
                FrameRate::Constant(0.1),
                Orientation::Horizontal,
            )?,
            attack_run_down_sword: r.load_sprite_sheet(
                Origin::Precise(Vec2::new(31.0, 31.0)),
                "player/attack_run_down_sword.png",
                3,
                FrameRate::Constant(0.1),
                Orientation::Horizontal,
            )?,
            attack_run_feet: r.load_sprite_sheet(
                Origin::Precise(Vec2::new(6.0, 7.0)),
                "player/attack_run_feet.png",
                6,
                FrameRate::Constant(0.1),
                Orientation::Horizontal,
            )?,
            attack_run_up_poncho: r.load_sprite_sheet(
                Origin::Precise(Vec2::new(31.0, 31.0)),
                "player/attack_run_up_poncho.png",
                3,
                FrameRate::Constant(0.1),
                Orientation::Horizontal,
            )?,
            attack_run_up_sword: r.load_sprite_sheet(
                Origin::Precise(Vec2::new(31.0, 31.0)),
                "player/attack_run_up_sword.png",
                3,
                FrameRate::Constant(0.1),
                Orientation::Horizontal,
            )?,
            attack_stand_down: r.load_sprite_sheet(
                Origin::Precise(Vec2::new(31.0, 31.0)),
                "player/attack_stand_down.png",
                3,
                FrameRate::Constant(0.1),
                Orientation::Horizontal,
            )?,
            attack_stand_up: r.load_sprite_sheet(
                Origin::Precise(Vec2::new(31.0, 31.0)),
                "player/attack_stand_up.png",
                3,
                FrameRate::Constant(0.1),
                Orientation::Horizontal,
            )?,
            attack_climb_down: r.load_sprite_sheet(
                Origin::Precise(Vec2::new(31.0, 31.0)),
                "player/attack_climb_down.png",
                3,
                FrameRate::Constant(0.1),
                Orientation::Horizontal,
            )?,
            attack_climb_up: r.load_sprite_sheet(
                Origin::Precise(Vec2::new(31.0, 31.0)),
                "player/attack_climb_up.png",
                3,
                FrameRate::Constant(0.1),
                Orientation::Horizontal,
            )?,
            slash: r.load_sprite_sheet(
                Origin::Precise(Vec2::new(-10.0, 15.0)),
                "effects/slash.png",
                3,
                FrameRate::None,
                Orientation::Horizontal,
            )?,
        };

        Ok(Self {
            state: State::Idle(IdleState::Standing),
            sprites,
            data: Data {
                flipped: false,
                health: 5,
                collision_rect: Rectangle {
                    x: -10.0,
                    y: -20.0,
                    w: 20.0,
                    h: 20.0,
                },
                velocity: Vec2::ZERO,
                position: 100.0 * Vec2::ONE,
                jump_count: 0,
                on_ground: false,
                direction: 0.0,
                queued_attack: false,
                attack_cooldown: 0.0,
                hurtbox: Rect {
                    x: -10.0,
                    y: -10.0,
                    w: 20.0,
                    h: 20.0,
                },
            },
        })
    }

    pub fn update(&mut self, delta: f32, input: &Input, level_geometry: &LevelGeometry) {
        let left_pressed = input.left.pressed;
        let right_pressed = input.right.pressed;
        self.data.direction = if !left_pressed && right_pressed {
            1
        } else if left_pressed && !right_pressed {
            -1
        } else {
            0
        } as f32;

        // update the player / state
        match &mut self.state {
            State::Idle(ref mut idle_state) => {
                match idle_state {
                    IdleState::Landing(ref mut time) => {
                        *time -= delta;
                    }
                    _ => {}
                }
                self.data.v_collide(delta, &level_geometry);
                self.data.jump_test(input);
                self.data.attack_test(delta, input);
            }
            State::Jump(_) => {
                self.data.apply_gravity(delta);
                self.data.attack_test(delta, input);
                self.data.update_velocity();

                self.data.v_collide(delta, &level_geometry);
                self.data.h_collide(delta, &level_geometry);
                self.data.update_pos(delta);

                self.data.jump_test(input);
            }
            State::Run(_) => {
                self.data.apply_gravity(delta);
                self.data.attack_test(delta, input);
                self.data.update_velocity();

                self.data.v_collide(delta, &level_geometry);
                self.data.h_collide(delta, &level_geometry);
                self.data.update_pos(delta);

                self.data.jump_test(input);

                //self.sprites.run.advance(delta);
            }
            State::AttackIdle(ref mut attack_state) => {
                attack_state.advance(delta);
                self.data.apply_gravity(delta);
                self.data.velocity.x = 80.0 * self.data.direction;
                self.data.v_collide(delta, &level_geometry);
                self.data.h_collide(delta, &level_geometry);
                self.data.update_pos(delta);
                self.sprites.update_attack_animations(attack_state);
            }
            State::AttackRun(ref mut attack_state) => {
                attack_state.advance(delta);
                self.data.apply_gravity(delta);
                self.data.v_collide(delta, &level_geometry);
                self.data.h_collide(delta, &level_geometry);
                self.data.update_pos(delta);
                //self.sprites.run.advance(delta);
                self.sprites.update_attack_animations(attack_state);
            }
            State::AttackJump(ref mut attack_state) => {
                attack_state.advance(delta);
                self.data.apply_gravity(delta);
                /*
                self.data.velocity = Vec2::new(
                    self.data.direction * ATTACK_AIR_MOVE_SPEED,
                    ATTACK_FALL_SPEED,
                );
                */
                self.data.v_collide(delta, &level_geometry);
                self.data.h_collide(delta, &level_geometry);
                self.data.update_pos(delta);
                self.sprites.update_attack_animations(attack_state);
            }
        };

        // Check for state changes
        loop {
            match &mut self.state {
                State::Idle(ref mut idle_state) => {
                    match idle_state {
                        IdleState::Landing(time) => {
                            *idle_state = if *time < 0.0 {
                                IdleState::Standing
                            } else {
                                IdleState::Landing(*time)
                            };
                        }
                        IdleState::Crouching => {
                            if !input.down.pressed {
                                *idle_state = IdleState::Standing;
                            }
                        }
                        IdleState::Standing => {
                            if input.down.pressed {
                                *idle_state = IdleState::Crouching;
                            }
                        }
                    }

                    if !self.data.on_ground {
                        self.state = State::Jump(JumpState::Fall);
                    } else if input.jump.just_pressed() {
                        self.state = State::Jump(JumpState::Rise);
                    } else if self.data.queued_attack {
                        self.data.queued_attack = false;
                        self.state = State::AttackIdle(AttackState {
                            time: ATTACK_DURATION,
                            stage: AttackStage::Up,
                        });
                    } else if self.data.direction != 0.0 {
                        self.state = State::Run(RunState::Start);
                    }
                }
                State::Jump(ref mut jump_state) => {
                    if self.data.on_ground {
                        self.state = State::Idle(if self.data.velocity.x.abs() < 0.1 {
                            IdleState::Landing(LAND_TIME)
                        } else {
                            IdleState::Standing
                        });
                    } else if self.data.queued_attack {
                        self.data.queued_attack = false;
                        self.state = State::AttackJump(AttackState {
                            time: ATTACK_DURATION,
                            stage: AttackStage::Up,
                        });
                    } else if self.data.velocity.y > 0.0 {
                        *jump_state = JumpState::Fall;
                    } else if self.data.velocity.y < 0.0 {
                        *jump_state = JumpState::Rise;
                    }
                }
                State::Run(ref mut run_state) => {
                    if !self.data.on_ground {
                        self.state = State::Jump(JumpState::Rise);
                    } else {
                        match run_state {
                            RunState::Start => {
                                if self.data.velocity.x.abs() >= H_SPEED * 0.5 {
                                    *run_state = RunState::Sprint;
                                }
                            }
                            RunState::Sprint => {
                                if self.data.velocity.x.abs() < 0.3 {
                                    self.state = State::Idle(IdleState::Standing);
                                }
                            }
                        }
                        if self.data.queued_attack {
                            self.data.queued_attack = false;
                            self.state = State::AttackRun(AttackState {
                                time: ATTACK_DURATION,
                                stage: AttackStage::Up,
                            });
                        }
                    }
                }
                State::AttackIdle(ref mut attack_state) => {
                    if attack_state.stage == AttackStage::Finish {
                        self.state = State::Idle(IdleState::Standing);
                    }
                }
                State::AttackRun(ref mut attack_state) => {
                    if attack_state.stage == AttackStage::Finish {
                        self.state = State::Run(RunState::Sprint);
                    }
                }
                State::AttackJump(ref mut attack_state) => {
                    if attack_state.stage == AttackStage::Finish {
                        self.state = State::Jump(JumpState::Fall);
                    }
                }
            }
            break;
        }

        // flipping logic
        self.data.flipped = if self.data.velocity.x > 0.0 {
            false
        } else if self.data.velocity.x < 0.0 {
            true
        } else {
            self.data.flipped
        };
    }

    pub fn draw(&self) -> DrawQueue {
        let mut dq = DrawQueue::new();
        let params = DrawParams {
            position: self.data.position,
            flip_x: self.data.flipped,
            flip_y: false,
            camera_locked: false,
        };
        match &self.state {
            State::Idle(idle_state) => match idle_state {
                IdleState::Landing(_) | IdleState::Crouching => {
                    dq.sprite(&self.sprites.jump_land, params);
                }
                IdleState::Standing => {
                    dq.sprite(&self.sprites.idle, params);
                }
            },
            State::Jump(jump_state) => match jump_state {
                JumpState::Rise => {
                    dq.sprite(&self.sprites.jump_rise, params);
                }
                JumpState::Fall => {
                    dq.sprite(&self.sprites.jump_fall, params);
                }
            },
            State::Run(run_state) => match run_state {
                RunState::Start => {
                    dq.sprite(&self.sprites.run_start, params);
                }
                RunState::Sprint => {
                    dq.sheet(&self.sprites.run, 0, params);
                }
            },
            State::AttackIdle(_attack_state) => {
                dq.sheet(&self.sprites.attack_stand_up, 0, params);
            }
            State::AttackRun(_attack_state) => {
                dq.sheet(&self.sprites.attack_run_up_sword, 0, params);
                dq.sheet(&self.sprites.attack_run_up_poncho, 0, params);
                dq.sheet(&self.sprites.attack_run_feet, 0, params);
            }
            State::AttackJump(_attack_state) => {
                dq.sheet(&self.sprites.attack_climb_up, 0, params);
            }
        }
        dq
    }
}

const GROUND_HEIGHT: f32 = 208 as f32;
const LAND_TIME: f32 = 0.100;
const H_SPEED: f32 = 200.0;
const GRAVITY: f32 = 2000.0;
const MAX_JUMPS: i8 = 2;
const JUMP_SPEED: f32 = 500.0;
const ATTACK_DURATION: f32 = 0.2;
const ATTACK_FALL_SPEED: f32 = 50.0;
const ATTACK_AIR_MOVE_SPEED: f32 = 50.0;

pub struct Data {
    pub health: i8,
    pub collision_rect: Rect,
    pub queued_attack: bool,
    pub attack_cooldown: f32,
    pub velocity: Vec2,
    pub position: Vec2,
    pub jump_count: i8,
    pub on_ground: bool,
    pub flipped: bool,
    pub direction: f32,
    pub hurtbox: Rect,
}

impl Data {
    /// Attack test is to check whether or not we should begin attacking
    /// during the update phase.
    pub fn attack_test(&mut self, delta: f32, input: &Input) {
        // deal with attack stuff, if we are attacking
        if input.attack.just_pressed() {
            self.queued_attack = true;
        } // now attack state is definitely None
        if self.attack_cooldown > 0.0 {
            self.attack_cooldown -= delta;
        }
        self.queued_attack = self.queued_attack && self.attack_cooldown <= 0.0;
    }

    pub fn update_velocity(&mut self) {
        if self.direction == 0.0 {
            self.velocity.x *= 0.3
        } else if self.direction.signum() == self.velocity.x.signum() {
            self.velocity.x += (self.direction * H_SPEED - self.velocity.x) * 0.5
        } else {
            self.velocity.x += (self.direction * H_SPEED - self.velocity.x) * 0.7
        };
    }

    pub fn apply_gravity(&mut self, delta: f32) {
        self.velocity.y += GRAVITY * delta;
    }
    pub fn jump_test(&mut self, input: &Input) {
        if
        /*self.jump_count > 0 &&*/
        input.jump.just_pressed() {
            self.velocity.y = -JUMP_SPEED;
            self.jump_count -= 1;
            self.on_ground = false;
        }
    }
    pub fn update_pos(&mut self, delta: f32) {
        self.position += self.velocity * delta;
    }

    fn collision(rect: &Rect, level_geometry: &LevelGeometry) -> bool {
        level_geometry.rectangles.iter().any(|r| r.contains(rect))
    }

    pub fn v_collide(&mut self, delta: f32, level_geometry: &LevelGeometry) {
        if Self::collision(
            &self
                .collision_rect
                .translate(self.position + Vec2::new(0.0, self.velocity.y * delta)),
            level_geometry,
        ) {
            self.position.y = if self.velocity.y > 0.0 {
                self.on_ground = true;
                self.position.y.floor()
            } else {
                self.position.y.ceil()
            };
            while !Self::collision(
                &self
                    .collision_rect
                    .translate(self.position + Vec2::new(0.0, self.velocity.y.signum())),
                level_geometry,
            ) {
                self.position.y += self.velocity.y.signum();
            }
            self.velocity.y = 0.0;
        }
    }

    pub fn h_collide(&mut self, delta: f32, level_geometry: &LevelGeometry) {
        if Self::collision(
            &self
                .collision_rect
                .translate(self.position + Vec2::new(self.velocity.x * delta, 0.0)),
            level_geometry,
        ) {
            self.position.x = if self.velocity.x > 0.0 {
                self.position.x.floor()
            } else {
                self.position.x.ceil()
            };
            while !Self::collision(
                &self
                    .collision_rect
                    .translate(self.position + Vec2::new(self.velocity.x.signum(), 0.0)),
                level_geometry,
            ) {
                self.position.x += self.velocity.x.signum();
            }
            self.velocity.x = 0.0;
        }
    }
}
