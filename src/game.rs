use crate::player::Player;
use glam::*;
use pyxl::{
    file_system::LoadError,
    graphics::{
        pixel_art::{sprite::*, sprite_sheet::*, tilemap::*},
        DrawParams, DrawQueue, Renderer,
    },
    input::*,
    math::Rect,
};
use winit::window::Window;

pub struct Ui {
    boss_bar: Sprite,
    boss_base: Sprite,
    player_bar: Sprite,
    player_base: Sprite,
    player_heart_full: Sprite,
    player_heart_half: Sprite,
}

mod boss {
    use pyxl::graphics::pixel_art::sprite_sheet::SpriteSheet;

    use super::*;
    pub struct Boss {
        pub sprites: Sprites,
    }

    pub struct Sprites {
        pub awake: Sprite,
        pub idle_arms_bottom: SpriteSheet,
        pub idle_arms_top: SpriteSheet,
        pub idle_body: SpriteSheet,
        pub idle_head: SpriteSheet,
        pub sleep: Sprite,
    }
}

use boss::Boss;

pub struct Game {
    pub renderer: Renderer,
    pub previous_time: f32,
    pub current_time: f32,
    pub delta_time: f32,
    pub player: Player,
    pub boss: Boss,
    pub ui: Ui,
    pub environment: Environment,
    pub tile_map: TileMap,
    pub level_geometry: LevelGeometry,
}

pub struct Environment {
    pub clouds: Sprite,
    pub moon: Sprite,
    pub platforms: Sprite,
}

pub struct LevelGeometry {
    pub blocks: Vec<Rect>,
    pub ladders: Vec<Rect>,
    pub top_ladders: Vec<Rect>,
}

impl LevelGeometry {
    pub fn colliding(&self, rect: Rect) -> bool {
        self.blocks.iter().any(|r| r.contains(&rect))
    }

    pub fn ladder_colliding(&self, rect: Rect) -> bool {
        self.ladders.iter().any(|r| r.contains(&rect))
    }

    pub fn top_ladder_colliding(&self, rect: Rect) -> bool {
        self.top_ladders.iter().any(|r| r.contains(&rect))
    }

    pub fn collisions(&self, rect: Rect) -> Vec<Rect> {
        self.blocks
            .iter()
            .filter(|r| r.contains(&rect))
            .map(|r| *r)
            .collect()
    }

    pub fn ladder_collisions(&self, rect: Rect) -> Vec<Rect> {
        self.ladders
            .iter()
            .filter(|r| r.contains(&rect))
            .map(|r| *r)
            .collect()
    }

    pub fn top_ladder_collisions(&self, rect: Rect) -> Vec<Rect> {
        self.top_ladders
            .iter()
            .filter(|r| r.contains(&rect))
            .map(|r| *r)
            .collect()
    }
}

pub const CAMERA_WIDTH: u32 = 427;
pub const CAMERA_HEIGHT: u32 = 240;

impl Game {
    pub async fn new(window: &Window) -> Result<Game, LoadError> {
        let mut r = Renderer::new(window, CAMERA_WIDTH, CAMERA_HEIGHT).await;

        let environment = Environment {
            clouds: r.load_sprite(Origin::TopLeft, "environment/clouds.png")?,
            moon: r.load_sprite(Origin::TopLeft, "environment/moon.png")?,
            platforms: r.load_sprite(Origin::TopLeft, "environment/platforms.png")?,
        };
        let ui = Ui {
            boss_bar: r.load_sprite(Origin::TopLeft, "ui/boss_bar.png")?,
            boss_base: r.load_sprite(Origin::TopLeft, "ui/boss_base.png")?,
            player_bar: r.load_sprite(Origin::TopLeft, "ui/player_bar.png")?,
            player_base: r.load_sprite(Origin::TopLeft, "ui/player_base.png")?,
            player_heart_full: r.load_sprite(Origin::TopLeft, "ui/player_heart_full.png")?,
            player_heart_half: r.load_sprite(Origin::TopLeft, "ui/player_heart_half.png")?,
        };

        let player = Player::new(&mut r)?;

        let sprites = boss::Sprites {
            awake: r.load_sprite(Origin::BottomMiddle, "twelve_string/awake.png")?,
            idle_arms_bottom: r.load_sprite_sheet(
                Origin::TopMiddle,
                "twelve_string/idle_arms_bottom.png",
                6,
                FrameRate::Constant(0.16),
                Orientation::Horizontal,
            )?,
            idle_arms_top: r.load_sprite_sheet(
                Origin::BottomMiddle,
                "twelve_string/idle_arms_top.png",
                6,
                FrameRate::Constant(0.16),
                Orientation::Horizontal,
            )?,
            idle_head: r.load_sprite_sheet(
                Origin::Center,
                "twelve_string/idle_head.png",
                4,
                FrameRate::Constant(0.16),
                Orientation::Horizontal,
            )?,
            idle_body: r.load_sprite_sheet(
                Origin::BottomMiddle,
                "twelve_string/idle_body.png",
                4,
                FrameRate::Constant(0.16),
                Orientation::Horizontal,
            )?,
            sleep: r.load_sprite(Origin::BottomMiddle, "twelve_string/sleep.png")?,
        };

        let boss = Boss { sprites };

        let tile_map = r.load_tilemap("tiles/untitled.tmx")?;

        let blocks = tile_map
            .tile_layers
            .get("Inter")
            .unwrap()
            .tiles
            .iter()
            .map(|Tile { x, y, .. }| Rect {
                x: (tile_map.tile_width as i32 * (*x)) as f32,
                y: (tile_map.tile_height as i32 * (*y)) as f32,
                w: (tile_map.tile_width) as f32,
                h: (tile_map.tile_height) as f32,
            })
            .collect();
        let ladder_width = 8.0;
        let ladder_tiles = &tile_map.tile_layers.get("Ladders").unwrap().tiles;
        let ladders: Vec<Rect> = ladder_tiles
            .iter()
            .map(|Tile { x, y, .. }| Rect {
                x: (tile_map.tile_width as f32) * (*x as f32 + 0.5) - ladder_width / 2.0,
                y: (tile_map.tile_height as i32 * (*y)) as f32,
                w: ladder_width,
                h: (tile_map.tile_height) as f32,
            })
            .collect();

        //TODO: this won't work for ladders on the same axis
        let top_ladders = ladder_tiles
            .iter()
            .enumerate()
            .filter(|(i, ladder_tile)| {
                if *i == 0 {
                    true
                } else {
                    let prev_tile = ladder_tiles.get(i - 1).unwrap();
                    prev_tile.y + 1 != ladder_tile.y || prev_tile.x != ladder_tile.x
                }
            })
            .map(|(_, Tile { x, y, .. })| Rect {
                x: (tile_map.tile_width as i32 * x) as f32,
                y: (tile_map.tile_height as i32 * y) as f32,
                w: (tile_map.tile_width) as f32,
                h: (tile_map.tile_height) as f32,
            })
            .collect();

        let level_geometry = LevelGeometry {
            blocks,
            ladders,
            top_ladders,
        };

        Ok(Self {
            renderer: r,
            player,
            ui,
            environment,
            boss,
            tile_map,
            level_geometry,
            previous_time: 0.0,
            current_time: 0.0,
            delta_time: 0.0,
        })
    }

    pub fn input(&mut self) {}

    pub fn update(&mut self, input: &Input, delta: f32) {
        self.player.update(delta, input, &self.level_geometry);

        self.renderer.update_camera(
            self.player.position - UVec2::new(CAMERA_WIDTH / 2, CAMERA_HEIGHT / 2).as_vec2(),
        );
    }

    pub fn draw(&mut self) {
        let mut dq = DrawQueue::new();

        // DRAW WORLD
        {
            dq.sprite(
                &self.environment.moon,
                DrawParams::from_pos(Vec2::new(320.0, 59.0)),
            );
            dq.sprite(&self.environment.clouds, DrawParams::default());

            dq.tile_layer(&self.tile_map, "Backing");
            dq.tile_image(&self.tile_map, "Clouds");
            dq.tile_image(&self.tile_map, "Moon");
            dq.tile_layer(&self.tile_map, "Mountains");
            dq.tile_layer(&self.tile_map, "Cave");
            dq.tile_layer(&self.tile_map, "Graves");
            dq.tile_layer(&self.tile_map, "Inter");
            dq.tile_layer(&self.tile_map, "Spikes");
            dq.tile_layer(&self.tile_map, "Ladders");
            dq.tile_layer(&self.tile_map, "Decorate");
        }

        // DRAW CHARACTERS
        {
            // draw twelve string
            dq.sheet(
                &self.boss.sprites.idle_body,
                0,
                DrawParams {
                    position: Vec2::new(218.0, 180.0),
                    flip_x: false,
                    flip_y: false,
                    camera_locked: false,
                },
            );
            dq.sheet(
                &self.boss.sprites.idle_arms_bottom,
                0,
                DrawParams {
                    position: Vec2::new(218.0, 55.0),
                    flip_x: false,
                    flip_y: false,
                    camera_locked: false,
                },
            );
            dq.sheet(
                &self.boss.sprites.idle_arms_top,
                0,
                DrawParams {
                    position: Vec2::new(218.0, 132.0),
                    flip_x: false,
                    flip_y: false,
                    camera_locked: false,
                },
            );
            dq.sheet(
                &self.boss.sprites.idle_head,
                0,
                DrawParams {
                    position: Vec2::new(218.0, 82.0),
                    flip_x: false,
                    flip_y: false,
                    camera_locked: false,
                },
            );

            // draw player
            dq.append(self.player.draw());
        }

        // DRAW UI
        {
            let player_base_x = 13.0;
            let player_base_y = 11.0;
            dq.sprite(
                &self.ui.player_base,
                DrawParams::from_pos(Vec2::new(player_base_x, player_base_y)).ui(true),
            );
            let heart_x = 11.0 + player_base_x;
            let heart_y = 19.0 + player_base_y;
            let heart_spacing = 21.0;
            for i in 0..self.player.health / 2 {
                dq.sprite(
                    &self.ui.player_heart_full,
                    DrawParams::from_pos(Vec2::new(heart_x + heart_spacing * (i as f32), heart_y))
                        .ui(true),
                );
            }
            if self.player.health % 2 == 1 {
                let x = heart_x + heart_spacing * (self.player.health / 2) as f32;
                dq.sprite(
                    &self.ui.player_heart_half,
                    DrawParams::from_pos(Vec2::new(x, heart_y)).ui(true),
                );
            }

            //draw bar, whatever it represents
            for i in 0..69 {
                dq.sprite(
                    &self.ui.player_bar,
                    DrawParams::from_pos(Vec2::new(
                        i as f32 + player_base_x + 4.0,
                        player_base_y + 4.0,
                    ))
                    .ui(true),
                );
            }
            let boss_bar_x = 78.0;
            let boss_bar_y = 211.0;
            dq.sprite(
                &self.ui.boss_base,
                DrawParams::from_pos(Vec2::new(boss_bar_x, boss_bar_y)).ui(true),
            );
            for i in 0..224 {
                dq.sprite(
                    &self.ui.boss_bar,
                    DrawParams::from_pos(Vec2::new(
                        i as f32 + boss_bar_x + 26.0,
                        boss_bar_y + 11.0,
                    ))
                    .ui(true),
                );
            }
        }

        self.renderer.render(dq);
    }
}
