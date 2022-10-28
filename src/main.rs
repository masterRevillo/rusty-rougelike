#[macro_use]
extern crate lazy_static;

use std::borrow::{Borrow, BorrowMut};
use std::cmp;
use std::collections::HashMap;

use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};

use log::LevelFilter;
use rand::distributions::{IndependentSample, Weighted, WeightedChoice};
use rand::Rng;
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;
use tcod::colors::*;
use tcod::console::*;
use tcod::input::{self, Event, Key, Mouse};
use tcod::input::KeyCode::Escape;
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
use crate::game_engine::GameEngine;
use map::mapgen::{in_map_bounds, LEVEL_TYPE_TRANSITION, make_boss_map, make_map, MAP_HEIGHT, MAP_WIDTH};
use map::mapgen::Map;
use graphics::render_functions::{initialize_fov, inventory_menu, menu, msgbox};
use crate::entities::entity_actions::{pick_item_up, player_move_or_attack};
use crate::entities::slot::Slot;
use crate::inventory::inventory_actions::{drop_item, use_item};
use crate::map::map_functions::next_level;
use crate::setup_game::main_menu;
use crate::map::tile::Tile;
use crate::util::ai::Ai;
use crate::util::ai::ai_take_turn;
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

// Takes 2 indexes of an array and returns the mutable item correspoding to both
// This is done by splitting the array into 2 mutable chunks, which contain the
// two desired items. The items are then indexed from the 2 slices, and then returned
fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index != second_index);
    if first_index < second_index {
        let (first_slice, second_slice) = items.split_at_mut(second_index);
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        let (first_slice, second_slice) = items.split_at_mut(first_index);
        (&mut second_slice[0], & mut first_slice[second_index])
    }
}

pub enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
}

fn target_tile(
    tcod: &mut Tcod,
    game: &mut GameEngine,
    max_range: Option<f32>
) -> Option<(i32, i32)> {
    use tcod::input::KeyCode::Escape;
    loop {
        tcod.root.flush();
        let event = input::check_for_event(input::KEY_PRESS | input::MOUSE).map(|e| e.1);
        match event {
            Some(Event::Mouse(m)) => tcod.mouse = m,
            Some(Event::Key(k)) => tcod.key = k,
            None => tcod.key = Default::default()
        }
        game.render_all(tcod, false);
        let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);

        let in_fov = (x < MAP_WIDTH) && (y < MAP_HEIGHT) && tcod.fov.is_in_fov(x, y);
        let in_range = max_range.map_or(true, |range| game.entities[PLAYER].distance(x, y) <= range);
        if tcod.mouse.lbutton_pressed && in_fov && in_range {
            return Some((x, y))
        } 
        if tcod.mouse.rbutton_pressed || tcod.key.code == Escape {
            return None;
        }
    }
}

fn handle_keys(tcod: &mut Tcod, game: &mut GameEngine) -> PlayerAction {
    use tcod::input::KeyCode::*;
    use PlayerAction::*;

    let player_alive = game.entities[PLAYER].alive;
    match (tcod.key, tcod.key.text(), player_alive) {
        (Key {code: Enter, alt: true, ..}, _, _,) => {               // the 2 dots signify that we dont care about the other values of Key. Without them, the code wouldnt compile until all values were supplied
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        }
        (Key { code: Escape, ..}, _, _, )=> return Exit,

        // movement keys
        (Key { code: Up, .. }, _, true ) | (Key { code: NumPad8, .. }, _, true ) => {
            player_move_or_attack(0, -1, game);
            TookTurn
        },
        (Key { code: Down, .. }, _, true ) | (Key { code: NumPad2, .. }, _, true ) => {
            player_move_or_attack(0, 1, game);
            TookTurn
        },
        (Key { code: Left, .. }, _, true ) | (Key { code: NumPad4, .. }, _, true ) => {
            player_move_or_attack(-1, 0, game);
            TookTurn
        },
        (Key { code: Right, .. }, _, true ) | (Key { code: NumPad6, .. }, _, true ) => {
            player_move_or_attack(1, 0, game);
            TookTurn
        },
        (Key { code: Home, .. }, _, true ) | (Key { code: NumPad7, .. }, _, true ) => {
            player_move_or_attack(-1, -1, game);
            TookTurn
        },
        (Key { code: PageUp, .. }, _, true ) | (Key { code: NumPad9, .. }, _, true ) => {
            player_move_or_attack(1, -1, game);
            TookTurn
        },
        (Key { code: End, .. }, _, true ) | (Key { code: NumPad1, .. }, _, true ) => {
            player_move_or_attack(-1, 1, game);
            TookTurn
        },
        (Key { code: PageDown, .. }, _, true ) | (Key { code: NumPad3, .. }, _, true ) => {
            player_move_or_attack(1, 1, game);
            TookTurn
        },
        (Key { code: NumPad5, .. }, _, true ) | (Key { code: Text, .. }, ".", true ) => {
            TookTurn
        },
        (Key { code: Text, .. }, "g", true ) => {
            let item_id = game.entities.iter().position(|object| object.pos() == game.entities[PLAYER].pos() && object.item.is_some());
            if let Some(item_id) = item_id {
                pick_item_up(item_id, game);
            }
            DidntTakeTurn
        },
        (Key { code: Text, .. }, "i", true ) => {
            let inventory_selection = inventory_menu(
                &game.entities[PLAYER].inventory, "Select an item to use by pressing the matching key, or any other to cancel\n",  &mut tcod.root
            );
            if let Some(inventory_selection) = inventory_selection {
                use_item(inventory_selection, tcod, game);
            }
            DidntTakeTurn
        },
        (Key {code: Text, ..}, "d", true ) => {
            let inventory_selection = inventory_menu(
                &game.entities[PLAYER].inventory, "Select an item you want to drop by pressing the matching key, or any other to cancel\n", &mut tcod.root
            );
            if let Some(inventory_selection) = inventory_selection {
                drop_item(inventory_selection, game);
            }
            DidntTakeTurn
        },
        (Key {code: Text, ..}, "<", true) => {
            let player_on_stairs = game.entities
            .iter()
            .any(|object| object.pos() == game.entities[PLAYER].pos() && object.name == "stairs");
            if player_on_stairs {
                next_level(tcod, game);
            }
            DidntTakeTurn
        },
        (Key {code: Text, ..}, "c", true) => {
            let player = &game.entities[PLAYER];
            let level = player.level;
            let level_up_xp = LEVEL_UP_BASE + level * LEVEL_UP_FACTOR;
            if let Some(fighter) = player.fighter.as_ref() {
                let msg = format!(
                    "Player stats: \n Level: {}\nExperience: {}\nExperience to level up: {}\n\nMaximum HP: {}\nAttack: {}\nbase_Defense: {}",
                    level, fighter.xp, level_up_xp, player.max_hp(), player.power(), player.defense()
                );
                msgbox(&msg, STATS_SCREEN_WIDTH, &mut tcod.root);
                
            }
            DidntTakeTurn
        }
        _ => DidntTakeTurn // everything else
    }

}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

pub fn run_game_loop(tcod: &mut Tcod, game: &mut GameEngine) {
    // for FOV recompute by setting player position to a weird value
    let mut previous_player_position = (-1, -1);

    while !tcod.root.window_closed() {
        // clear offscreen console before drawing anything
        tcod.con.clear();

        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => tcod.key = k,
            _ => tcod.key = Default::default(),
        }
        
        let fov_recompute = previous_player_position != (game.entities[PLAYER].pos());
        
        game.render_all(tcod, fov_recompute);
        
        tcod.root.flush();

        level_up(tcod, game.entities[PLAYER].borrow_mut());
        
        previous_player_position = game.entities[PLAYER].pos();
        let player_action = handle_keys(tcod, game);
        if player_action == PlayerAction::Exit {
            save_game(game).unwrap();
            break;
        }
        game.process_events();

        if game.entities[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..game.entities.len() {
                if game.entities[id].ai.is_some() {
                    ai_take_turn(id, &tcod, game)
                }
            }
        }
    }
}

// return type is a result, which can either be a success, or a type that implements the error type.
fn save_game(game: &mut GameEngine) -> Result<(), Box<dyn Error>> {
    let save_data = serde_json::to_string(&game)?;       // the ? gets the success value, or returns immediately with the error type
    let mut file = File::create("savegame")?;
    file.write_all(save_data.as_bytes())?;
    Ok(())
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
