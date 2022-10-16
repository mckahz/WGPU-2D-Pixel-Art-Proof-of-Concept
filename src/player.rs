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
pub struct Player {
    pub state: State,

    pub sprites: Sprites,
    pub run_t: AnimationTime,

    pub health: i8,

    pub collision_rect: Rect,

    pub velocity: Vec2,
    pub position: Vec2,

    pub jump_count: i8,
    pub off_ground_timer: f32,
    pub on_ground_timer: f32,

    pub flipped: bool,
    pub direction: f32,
    pub hurtbox: Rect,

    pub top_of_ladder: bool,
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
const BOUNCE_TIME: f32 = 0.15;
const CLIMB_SPEED: f32 = 150.0;
const CLIMB_END_DISTANCE: f32 = 14.0;
const CLIMB_FLIP_DISTANCE: f32 = 24.0;

#[derive(PartialEq)]
pub enum State {
    Land(f32),
    Idle,
    RunStart,
    Run,
    Jump,
    Attack,
    Climb,
}

impl Player {
    pub fn new(r: &mut Renderer) -> Result<Self, LoadError> {
        fn origin() -> Origin {
            Origin::Precise(Vec2::new(31.0, 22.0))
        }
        let sprites = Sprites {
            idle: r.load_sprite(origin(), "player/idle.png")?,
            jump_fall: r.load_sprite(origin(), "player/jump_fall.png")?,
            jump_land: r.load_sprite(origin(), "player/jump_land.png")?,
            jump_rise: r.load_sprite(origin(), "player/jump_rise.png")?,
            run_start: r.load_sprite(origin(), "player/run_start.png")?,
            climb: r.load_sprite(Origin::BottomMiddle, "player/climb.png")?,
            climb_end: r.load_sprite(Origin::BottomMiddle, "player/climb_end.png")?,
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

        let run_t = AnimationTime::new(&sprites.run);

        let rect_width = 6.0;
        let rect_height = 10.0;

        Ok(Self {
            state: State::Idle,
            sprites,
            run_t,
            flipped: false,
            health: 5,
            collision_rect: Rectangle {
                x: -rect_width / 2.0,
                y: -rect_height,
                w: rect_width,
                h: rect_height,
            },
            velocity: Vec2::ZERO,
            position: 100.0 * Vec2::ONE,
            jump_count: 0,
            off_ground_timer: 0.0,
            on_ground_timer: 0.0,
            direction: 0.0,
            hurtbox: Rect {
                x: -10.0,
                y: -10.0,
                w: 20.0,
                h: 20.0,
            },
            top_of_ladder: false,
        })
    }

    fn set_velocity_and_flip(&mut self, input: &Input) {
        self.direction = (if input.right.pressed { 1 } else { 0 }
            - if input.left.pressed { 1 } else { 0 }) as f32;

        self.velocity.x = self.direction * H_SPEED;
        if self.velocity.x < 0.0 {
            self.flipped = true;
        } else if self.velocity.x > 0.0 {
            self.flipped = false;
        }
    }

    fn gravity(&mut self, delta: f32) {
        self.velocity.y += GRAVITY * delta;
    }

    fn jump(&mut self, input: &Input) {
        if input.jump.just_pressed() {
            if self.on_ground() {
                self.velocity.y = -JUMP_SPEED;
            } else if self.state == State::Climb {
                self.state = State::Jump;
                self.velocity.y = -JUMP_SPEED;
            }
        }
    }

    fn climb(&mut self, input: &Input, level_geometry: &LevelGeometry) {
        let global_rect = self.global_rect();
        let ladder_below = level_geometry.ladder_colliding(global_rect.translate_y(1.0));
        let in_ladder = level_geometry.ladder_colliding(global_rect);
        let mut snap_to_ladder = || {
            self.position.x = (self.position.x / 16.0).floor() * 16.0 + 8.0;
            self.velocity.x = 0.0;
            self.state = State::Climb;
        };
        if in_ladder && input.up.pressed {
            snap_to_ladder();
            self.position.y -= 1.0;
        }
        if ladder_below
            && input.down.pressed
            && !level_geometry.colliding(global_rect.translate_y(1.0))
        {
            snap_to_ladder();
            self.position.y += 1.0;
        }
    }

    fn move_and_collide(&mut self, delta: f32, level_geometry: &LevelGeometry) {
        if level_geometry.colliding(self.global_rect().translate_x(self.velocity.x * delta)) {
            self.position.x = self.position.x.floor();
            while !level_geometry
                .colliding(self.global_rect().translate_x(self.velocity.x.signum()))
            {
                self.position.x += self.velocity.x.signum();
            }
            self.velocity.x = 0.0;
        }
        self.position.x += self.velocity.x * delta;

        if level_geometry.colliding(self.global_rect().translate_y(self.velocity.y * delta)) {
            self.position.y = self.position.y.floor();

            while !level_geometry
                .colliding(self.global_rect().translate_y(self.velocity.y.signum()))
            {
                self.position.y += self.velocity.y.signum();
            }
            self.velocity.y = 0.0;
        }
        if self.velocity.y > 0.0
            && !level_geometry.top_ladder_colliding(self.global_rect())
            && level_geometry
                .top_ladder_colliding(self.global_rect().translate_y(self.velocity.y * delta))
        {
            self.position.y = self.position.y.floor();

            while !level_geometry.top_ladder_colliding(self.global_rect().translate_y(1.0)) {
                self.position.y += 1.0;
            }
            self.velocity.y = 0.0;
        }
        self.position.y += self.velocity.y * delta;

        if level_geometry.colliding(self.global_rect().translate_y(1.0))
            || (!level_geometry.top_ladder_colliding(self.global_rect())
                && level_geometry.top_ladder_colliding(self.global_rect().translate_y(1.0)))
        {
            self.off_ground_timer = 0.0;
            self.on_ground_timer = f32::min(self.on_ground_timer + delta, BOUNCE_TIME);
        } else {
            self.off_ground_timer += delta;
            self.on_ground_timer = 0.0;
        }
    }

    pub fn update(&mut self, delta: f32, input: &Input, level_geometry: &LevelGeometry) {
        match &mut self.state {
            State::Idle => {
                if self.velocity.x != 0.0 {
                    self.state = State::RunStart;
                }
                if !self.on_ground() {
                    self.state = State::Jump;
                }
                self.set_velocity_and_flip(input);
                self.climb(input, level_geometry);
                self.jump(input);
                self.gravity(delta);
            }
            State::Land(ref mut bounce_time) => {
                *bounce_time -= delta;
                if self.velocity.x != 0.0 {
                    self.state = State::RunStart;
                } else if *bounce_time <= 0.0 {
                    self.state = State::Idle;
                }
                if !self.on_ground() {
                    self.state = State::Jump;
                }
                self.set_velocity_and_flip(input);
                self.climb(input, level_geometry);
                self.jump(input);
                self.gravity(delta);
            }
            State::RunStart => {
                if self.velocity.x.abs() == H_SPEED {
                    self.state = State::Run;
                }
                if !self.on_ground() {
                    self.state = State::Jump;
                }
                self.set_velocity_and_flip(input);
                self.climb(input, level_geometry);
                self.jump(input);
                self.gravity(delta);
            }
            State::Run => {
                self.set_velocity_and_flip(input);
                self.climb(input, level_geometry);
                if self.velocity.x == 0.0 {
                    self.state = State::Idle;
                }
                if !self.on_ground() {
                    self.state = State::Jump;
                }
                self.jump(input);
                self.gravity(delta);
            }
            State::Jump => {
                self.set_velocity_and_flip(input);
                self.climb(input, level_geometry);
                if self.on_ground() {
                    self.state = State::Land(BOUNCE_TIME);
                }
                self.jump(input);
                self.gravity(delta);
            }
            State::Attack => todo!(),
            State::Climb => {
                let climb_direction = (if input.down.pressed { 1 } else { 0 }
                    - if input.up.pressed { 1 } else { 0 })
                    as f32;
                self.velocity.y = climb_direction * CLIMB_SPEED;

                if !level_geometry.ladder_colliding(self.global_rect()) {
                    self.velocity.y = 0.0;
                    self.state = State::Idle;
                    self.position.y = ((self.position.y / 16.0).floor() + 1.0) * 16.0;
                }
                self.top_of_ladder = !level_geometry
                    .ladder_colliding(self.global_rect().translate_y(-CLIMB_END_DISTANCE));
                if level_geometry.colliding(self.global_rect().translate_y(1.0))
                    && climb_direction == 1.0
                {
                    self.state = State::Idle;
                }

                self.jump(input);
            }
        }

        self.move_and_collide(delta, level_geometry);
        self.run_t.advance(delta);
    }

    pub fn draw(&self) -> DrawQueue {
        let mut dq = DrawQueue::new();
        let mut params = DrawParams {
            position: self.position,
            flip_x: self.flipped,
            flip_y: false,
            camera_locked: false,
        };

        match &self.state {
            State::Land(_) => dq.sprite(&self.sprites.jump_land, params),
            State::Idle => dq.sprite(&self.sprites.idle, params),
            State::RunStart => dq.sprite(&self.sprites.run_start, params),
            State::Run => dq.sheet(&self.sprites.run, self.run_t.frame(), params),
            State::Jump => {
                if self.velocity.y <= 0.0 {
                    dq.sprite(&self.sprites.jump_rise, params);
                } else {
                    dq.sprite(&self.sprites.jump_fall, params);
                }
            }
            State::Attack => todo!(),
            State::Climb => {
                params.flip_x = (self.position.y / CLIMB_FLIP_DISTANCE).floor() % 2.0 == 0.0;
                if self.top_of_ladder {
                    dq.sprite(&self.sprites.climb_end, params);
                } else {
                    dq.sprite(&self.sprites.climb, params);
                }
            }
        }

        dq
    }

    fn global_rect(&self) -> Rect {
        self.collision_rect.translate(self.position)
    }

    fn on_ground(&self) -> bool {
        self.on_ground_timer > 0.0
    }
}
