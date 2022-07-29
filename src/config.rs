use configparser::ini::Ini;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GameConfig {
    pub play_sfx: bool,
    pub sfx_volume: f32,
    pub play_bgm: bool,
    pub bgm_volume: f32
}

pub fn load_configs() -> GameConfig {
    let mut config = Ini::new();
    config.load("properties.ini");
    GameConfig {
        play_sfx: config.getbool("audio", "play_sfx").unwrap().unwrap_or(true),
        sfx_volume: config.getfloat("audio", "sfx_volume").unwrap().unwrap_or(0.0) as f32,
        play_bgm: config.getbool("audio", "play_bgm").unwrap().unwrap_or(true),
        bgm_volume: config.getfloat("audio", "bgm_volume").unwrap().unwrap_or(0.0) as f32
    }
}