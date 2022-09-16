#![allow(dead_code)]

use crate::{
    graphics::{sprite::*, DrawParams, DrawQueue, Renderer},
    {input::*, math::*},
};
use glam::*;
use winit::window::Window;

pub struct Ui {
    boss_bar: Sprite,
    boss_base: Sprite,
    player_bar: Sprite,
    player_base: Sprite,
    player_heart_full: Sprite,
    player_heart_half: Sprite,
}

use crate::player::{self, Player};

pub struct Game {
    pub renderer: Renderer,
    pub previous_time: f32,
    pub current_time: f32,
    pub delta_time: f32,
    pub player: Player,
    pub ui: Ui,
    pub environment: Environment,
}

pub struct Environment {
    pub clouds: Sprite,
    pub moon: Sprite,
    pub platforms: Sprite,
}

pub const CAMERA_WIDTH: u32 = 427;
pub const CAMERA_HEIGHT: u32 = 240;

impl Game {
    pub async fn new(window: &Window) -> Game {
        let mut r = Renderer::new(window, CAMERA_WIDTH, CAMERA_HEIGHT).await;

        let (environment, ui, player_sprites, player_sprite_sheets) = {
            let environment = Environment {
                clouds: r
                    .load_sprite(Origin::TopLeft, "environment/clouds.png")
                    .unwrap(),
                moon: r
                    .load_sprite(Origin::TopLeft, "environment/moon.png")
                    .unwrap(),
                platforms: r
                    .load_sprite(Origin::TopLeft, "environment/platforms.png")
                    .unwrap(),
            };
            let ui = Ui {
                boss_bar: r.load_sprite(Origin::TopLeft, "ui/boss_bar.png").unwrap(),
                boss_base: r.load_sprite(Origin::TopLeft, "ui/boss_base.png").unwrap(),
                player_bar: r.load_sprite(Origin::TopLeft, "ui/player_bar.png").unwrap(),
                player_base: r
                    .load_sprite(Origin::TopLeft, "ui/player_base.png")
                    .unwrap(),
                player_heart_full: r
                    .load_sprite(Origin::TopLeft, "ui/player_heart_full.png")
                    .unwrap(),
                player_heart_half: r
                    .load_sprite(Origin::TopLeft, "ui/player_heart_half.png")
                    .unwrap(),
            };

            fn player_origin() -> Origin {
                Origin::Precise(Vec2::new(31.0, 22.0))
            }
            let player_sprites = player::Sprites {
                idle: r.load_sprite(player_origin(), "player/idle.png").unwrap(),
                jump_fall: r
                    .load_sprite(player_origin(), "player/jump_fall.png")
                    .unwrap(),
                jump_land: r
                    .load_sprite(player_origin(), "player/jump_land.png")
                    .unwrap(),
                jump_rise: r
                    .load_sprite(player_origin(), "player/jump_rise.png")
                    .unwrap(),
                run_start: r
                    .load_sprite(player_origin(), "player/run_start.png")
                    .unwrap(),
            };
            let player_sprite_sheets = player::SpriteSheets {
                run: r
                    .load_sprite_sheet(
                        Origin::Precise(Vec2::new(31.0, 23.0)),
                        "player/run.png",
                        6,
                        FrameRate::Constant(0.1),
                        Orientation::Horizontal,
                    )
                    .unwrap(),
            };
            (environment, ui, player_sprites, player_sprite_sheets)
        };

        let player = Player {
            state: player::State::Idle,
            sprites: player_sprites,
            sprite_sheets: player_sprite_sheets,
            data: player::Data {
                flipped: false,
                health: 5,
                collision_rect: Rectangle {
                    x: -10.0,
                    y: -10.0,
                    w: 20.0,
                    h: 20.0,
                },
                velocity: Vec2::ZERO,
                position: 100.0 * Vec2::ONE,
                jump_count: 0,
                on_ground: false,
            },
        };

        Self {
            renderer: r,
            previous_time: 0.0,
            current_time: 0.0,
            delta_time: 0.0,
            player,
            ui,
            environment,
        }
    }

    pub fn input(&mut self) {}

    pub fn update(&mut self, input: &Input, delta: f32) {
        self.player.update(delta, input);
    }

    pub fn draw(&mut self) {
        let mut dq = DrawQueue::new();

        // DRAW WORLD
        {
            dq.draw_sprite(
                &self.environment.moon,
                DrawParams::from_pos(Vec2::new(320.0, 59.0)),
            );
            dq.draw_sprite(&self.environment.clouds, DrawParams::default());
            dq.draw_sprite(&self.environment.platforms, DrawParams::default());
        }

        // DRAW CHARACTERS
        {
            // draw player
            let params = DrawParams {
                position: self.player.data.position,
                flip_x: self.player.data.flipped,
                flip_y: false,
            };
            match &self.player.state {
                player::State::Run(state) => match state {
                    player::RunState::Sprint => {
                        dq.draw_sprite_sheet(&self.player.sprite_sheets.run, params);
                    }
                    player::RunState::Start => {
                        dq.draw_sprite(&self.player.sprites.run_start, params);
                    }
                },
                player::State::Idle => {
                    dq.draw_sprite(&self.player.sprites.idle, params);
                }
                player::State::Jumping(state) => match state {
                    player::JumpState::Rise => {
                        dq.draw_sprite(&self.player.sprites.jump_rise, params);
                    }
                    player::JumpState::Land => {
                        dq.draw_sprite(&self.player.sprites.jump_land, params);
                    }
                    player::JumpState::Fall => {
                        dq.draw_sprite(&self.player.sprites.jump_fall, params);
                    }
                },
            };
        }

        // DRAW UI
        {
            let player_base_x = 13.0;
            let player_base_y = 11.0;
            dq.draw_sprite(
                &self.ui.player_base,
                DrawParams::from_pos(Vec2::new(player_base_x, player_base_y)),
            );
            let heart_x = 11.0 + player_base_x;
            let heart_y = 19.0 + player_base_y;
            let heart_spacing = 21.0;
            for i in 0..self.player.data.health / 2 {
                dq.draw_sprite(
                    &self.ui.player_heart_full,
                    DrawParams::from_pos(Vec2::new(heart_x + heart_spacing * (i as f32), heart_y)),
                );
            }
            if self.player.data.health % 2 == 1 {
                let x = heart_x + heart_spacing * (self.player.data.health / 2) as f32;
                dq.draw_sprite(
                    &self.ui.player_heart_half,
                    DrawParams::from_pos(Vec2::new(x, heart_y)),
                );
            }

            //draw bar, whatever it represents
            for i in 0..69 {
                dq.draw_sprite(
                    &self.ui.player_bar,
                    DrawParams::from_pos(Vec2::new(
                        i as f32 + player_base_x + 4.0,
                        player_base_y + 4.0,
                    )),
                );
            }
            let boss_bar_x = 78.0;
            let boss_bar_y = 211.0;
            dq.draw_sprite(
                &self.ui.boss_base,
                DrawParams::from_pos(Vec2::new(boss_bar_x, boss_bar_y)),
            );
            //224
            for i in 0..224 {
                dq.draw_sprite(
                    &self.ui.boss_bar,
                    DrawParams::from_pos(Vec2::new(
                        i as f32 + boss_bar_x + 26.0,
                        boss_bar_y + 11.0,
                    )),
                );
            }
        }

        self.renderer.render(dq);
    }
}
