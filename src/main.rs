#[macro_use]
extern crate lazy_static;

use log::LevelFilter;
use rand::distributions::{IndependentSample, Weighted, WeightedChoice};
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;
use tcod::console::*;
use tcod::map::{FovAlgorithm, Map as FovMap};
use events::audio_event_processor::AudioEventProcessor;
use events::event_log_processor::EventLogProcessor;
use events::game_occurrence::GameOccurrenceEventProcessor;
use util::death_callback::DeathCallback;
use util::messages::Messages;
use crate::graphics::camera::Camera;
use crate::config::game_config::{GameConfig, load_configs};
use entities::entity::Entity;
use crate::events::game_event_processing::{EventBus, EventData, EventProcessor, EventType, GameEvent};
use crate::framework::Tcod;
use crate::game_engine::{GameEngine, run_game_loop};
use map::mapgen::{in_map_bounds, make_map, MAP_HEIGHT, MAP_WIDTH};
use map::mapgen::Map;
use graphics::render_functions::{initialize_fov, menu, msgbox};
use crate::setup_game::{main_menu, save_game};
use crate::util::transition::Transition;

mod events {
    pub mod game_event_processing;
    pub mod audio_event_processor;
    pub mod game_occurrence;
    pub mod event_log_processor;
}
mod entities {
    pub mod entity;
    pub mod fighter;
    pub mod equipment;
    pub mod slot;
    pub mod entity_actions;
}
mod config {
    pub mod game_config;
}
mod map {
    pub mod mapgen;
    pub mod tile;
    pub mod map_functions;
}
mod graphics {
    pub mod camera;
    pub mod render_functions;
}

mod inventory {
    pub mod inventory_actions;
}

mod game_engine;
mod framework;
mod setup_game;

mod audio {
    pub mod audio_engine;
}
mod util {
    pub mod ai;
    pub mod transition;
    pub mod death_callback;
    pub mod namegen;
    pub mod messages;
    pub mod mut_two;
}


const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 75;

const INVENTORY_WIDTH: i32 = 50;

const LIMIT_FPS: i32 = 20;

const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;

const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

const PLAYER: usize = 0;

//fov settings
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;

//parameters for items
const HEAL_AMOUNT: i32 = 4;
const LIGHTNING_DAMAGE: i32 = 40;
const LIGHTNING_RANGE: i32 = 5;
const CONFUSE_RANGE: i32 = 8;
const CONFUSE_NUM_TURNS: i32 = 10;
const FIREBALL_RADIUS: i32 = 3;
const FIREBALL_DAMAGE: i32 = 12;

//parameters for leveling up
const LEVEL_UP_BASE: i32 = 200;
const LEVEL_UP_FACTOR: i32 = 150;
const LEVEL_SCREEN_WIDTH: i32 = 40;
const STATS_SCREEN_WIDTH: i32 = 30;

fn level_up(tcod: &mut Tcod, player: &mut Entity) {
    let level_up_xp = LEVEL_UP_BASE + LEVEL_UP_FACTOR * player.level;
    if player.fighter.as_ref().map_or(0, |f| f.xp) >= level_up_xp {
        player.level += 1;
        //TODO add message back in:
        // game.messages.add(format!("Your experience has increased. You are now level {}!", player.level), YELLOW);
        let fighter = player.fighter.as_mut().unwrap();
        let mut choice = None;
        while choice.is_none() {
            choice = menu(
                "Level up! Choose a stat to increase: \n",
                &[
                    format!("Constitution (+20 HP, from {})", fighter.base_max_hp),
                    format!("Strength (+1 attack, from {})", fighter.base_power),
                    format!("Agility (+1 defense, from {})", fighter.base_defense),
                ],
                LEVEL_SCREEN_WIDTH,
                &mut tcod.root
            )
        }
        tcod.root.flush();
        fighter.xp -= level_up_xp;
        match choice.unwrap() {
            0 => {
                fighter.base_max_hp += 20;
                fighter.hp += 20;
            }
            1 => {
                fighter.base_power += 1;
            }
            2 => {
                fighter.base_defense += 1;
            }
            _ => unreachable!()
        }
    } 
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Heal,
    Lightning,
    Confuse,
    Fireball,
    Artifact {name: String, value: i32},
    Sword,
    Shield,
}

fn from_dungeon_level(table: &[Transition], level: u32) -> u32 {
    table
        .iter()
        .rev()
        .find(|transition| level >= transition.level)
        .map_or(0, |transition| transition.value)
}

pub enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
}




lazy_static! {
    static ref GAME_CONFIGS: GameConfig = load_configs();
}

fn main() {
    SimpleLogger::new()
        .with_colors(true)
        .with_level(LevelFilter::Info)
        .init().unwrap();

    tcod::system::set_fps(LIMIT_FPS);

    let root = Root::initializer()
        .font("consolas12x12_gs_tc.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("A Rusty Rougelike")
        .init();

    let mut tcod = Tcod {
        root, 
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
        key: Default::default(),                    // default is a trait that can be implemented that gives an object default values
        mouse: Default::default()
    };

    main_menu(&mut tcod);
}
