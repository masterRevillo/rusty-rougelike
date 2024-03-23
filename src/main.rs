#[macro_use]
extern crate lazy_static;

use bracket_lib::prelude::{BError, BTerm, GameState, main_loop};
use bracket_lib::terminal::BTermBuilder;
use log::LevelFilter;
use rand::distributions::{IndependentSample, Weighted, WeightedChoice};
use simple_logger::SimpleLogger;
// use tcod::console::*;
// use tcod::map::Map as FovMap;

use entities::entity::Entity;
use events::audio_event_processor::AudioEventProcessor;
use events::event_log_processor::EventLogProcessor;
use events::game_occurrence::GameOccurrenceEventProcessor;
use graphics::render_functions::{initialize_fov, menu, msgbox};
use map::mapgen::{in_map_bounds, make_map, MAP_HEIGHT, MAP_WIDTH};
use map::mapgen::Map;
use util::death_callback::DeathCallback;
use util::messages::Messages;

use crate::config::game_config::{GameConfig, load_configs};
use crate::events::game_event_processing::{EventBus, EventData, EventProcessor, EventType, GameEvent};
use crate::framework::GameFramework;
use crate::game_engine::{GameEngine, StateType};
use crate::graphics::camera::Camera;
use crate::setup_game::{save_game};
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
mod items {
    pub mod item;
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

const LIMIT_FPS: i32 = 20;

lazy_static! {
    static ref GAME_CONFIGS: GameConfig = load_configs();
}

struct State {
    pub current_state: StateType,
    pub engine: GameEngine
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut BTerm) {
        ctx.cls();
        ctx.print(1, 1, "Hello Rust World");
    }
}

fn main() -> BError{
    SimpleLogger::new()
        .with_colors(true)
        .with_level(LevelFilter::Info)
        .init().unwrap();

    // tcod::system::set_fps(LIMIT_FPS);

    let console = BTermBuilder::simple(SCREEN_WIDTH, SCREEN_HEIGHT).unwrap()
        // .with_font("consolas12x12_gs_tc.png", 12, 12)
        .with_title("A Rusty Rougelike")
        .build()?;

    let gs = State{
        current_state: StateType::MainMenu,

    };
    main_loop(console, gs)



    // let root = Root::initializer()
    //     .font("consolas12x12_gs_tc.png", FontLayout::Tcod)
    //     .font_type(FontType::Greyscale)
    //     .size(SCREEN_WIDTH, SCREEN_HEIGHT)
    //     .title("A Rusty Rougelike")
    //     .init();
    //
    // let mut tcod = Tcod {
    //     root,
    //     con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
    //     panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
    //     fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
    //     key: Default::default(),                    // default is a trait that can be implemented that gives an object default values
    //     mouse: Default::default()
    // };
    //
    // main_menu(&mut tcod);
}
