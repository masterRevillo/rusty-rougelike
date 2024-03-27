use serde::{Deserialize, Serialize};
use rand::Rng;
use crate::util::color::{Color, DARK_RED};


const COLOR_DARK_WALL_SURFACE: Color = Color{r: 43.0, g: 0.0, b: 0.0};
const COLOR_DARK_WALL: Color = Color{r: 139.0, g: 0.0, b: 0.0};
const COLOR_LIGHT_WALL_SURFACE: Color = Color{r: 93.0, g: 10.0, b: 10.0};
const COLOR_LIGHT_WALL: Color = Color {r: 127.0, g: 30.0, b: 20.0};
const COLOR_DARK_GROUND_SURFACE: Color = Color {r: 15.0,g: 8.0,b: 8.0,};
const COLOR_DARK_GROUND: Color = Color {r: 20.0,g: 10.0,b: 10.0,};
const COLOR_LIGHT_GROUND_SURFACE: Color = Color {r: 150.0, g: 101.0, b: 90.0};
const COLOR_LIGHT_GROUND: Color = Color {r: 170.0, g: 131.0, b: 96.0};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TileType {
    Ground,
    Wall
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]   // This allows the struct to implement some default behaviors provided by Rust. They are called "traits", but evidently they can be thought of like interfaces
pub struct Tile {                   // Debug lets us print out the value of the struct; Clone and Copy overrides default assignment strategy of "moving"
    pub tile_type: TileType,
    pub(crate) blocked: bool,
    pub block_sight: bool,
    pub explored: bool,
    pub lit_color: Color,
    pub dark_color: Color,
    pub surface_char: char,
    pub surface_lit_color: Color,
    pub surface_dark_color: Color,
}

impl Tile {
    pub fn ground() -> Self {
        let mut rng = rand::thread_rng();
        let x = rng.gen::<f64>();

        let c = match x {
            x if x > 0.95 => ('(', COLOR_LIGHT_GROUND_SURFACE, COLOR_DARK_GROUND_SURFACE),
            x if x > 0.9 => ('-', COLOR_LIGHT_GROUND_SURFACE, COLOR_DARK_GROUND_SURFACE),
            x if x > 0.85 => ('"', COLOR_LIGHT_GROUND_SURFACE, COLOR_DARK_GROUND_SURFACE),
            _ => ('.', COLOR_LIGHT_GROUND, COLOR_DARK_GROUND)
        };

        Tile {
            tile_type: TileType::Ground,
            blocked: false,
            block_sight: false,
            explored: false,
            lit_color: COLOR_LIGHT_GROUND,
            dark_color: COLOR_DARK_GROUND,
            surface_char: c.0,
            surface_lit_color: c.1,
            surface_dark_color: c.2
        }
    }

    pub fn wall() -> Self {
        let mut rng = rand::thread_rng();
        let x = rng.gen::<f64>();

        let c = match x {
            x if x > 0.95 => ('.', COLOR_LIGHT_WALL_SURFACE, COLOR_DARK_WALL_SURFACE),
            x if x > 0.9 => ('#', COLOR_LIGHT_WALL_SURFACE, COLOR_DARK_WALL_SURFACE),
            x if x > 0.85 => (':', COLOR_LIGHT_WALL_SURFACE, COLOR_DARK_WALL_SURFACE),
            x if x > 0.8 => ('/', COLOR_LIGHT_WALL_SURFACE, COLOR_DARK_WALL_SURFACE),
            x if x > 0.75 => ('`', COLOR_LIGHT_WALL_SURFACE, COLOR_DARK_WALL_SURFACE),
            x if x > 0.7 => ('*', COLOR_LIGHT_WALL_SURFACE, COLOR_DARK_WALL_SURFACE),
            x if x > 0.65 => ('%', COLOR_LIGHT_WALL_SURFACE, COLOR_DARK_WALL_SURFACE),
            x if x > 0.6 => ('\'', COLOR_LIGHT_WALL_SURFACE, COLOR_DARK_WALL_SURFACE),
            x if x > 0.55 => ('^', COLOR_LIGHT_WALL_SURFACE, COLOR_DARK_WALL_SURFACE),
            x if x > 0.5 => ('[', COLOR_LIGHT_WALL_SURFACE, COLOR_DARK_WALL_SURFACE),
            _ => ('.', COLOR_LIGHT_WALL, COLOR_DARK_WALL)
        };
        Tile {
            tile_type: TileType::Wall,
            blocked: true,
            block_sight: true,
            explored: false,
            lit_color: COLOR_LIGHT_WALL,
            dark_color: COLOR_DARK_WALL,
            surface_char: c.0,
            surface_lit_color: c.1,
            surface_dark_color: c.2
        }
    }
}
