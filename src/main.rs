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


use event_processing::audio_event_processor::AudioEventProcessor;
use event_processing::event_log_processor::EventLogProcessor;
use event_processing::game_occurrence::GameOccurrenceEventProcessor;
use util::death_callback::DeathCallback;

use crate::graphics::camera::Camera;
use crate::config::game_config::{GameConfig, load_configs};
use crate::entity::Entity;
use crate::event_processing::game_event_processing::{EventBus, EventData, EventProcessor, EventType, GameEvent};
use crate::framework::Tcod;
use crate::game_engine::GameEngine;
use crate::map::{in_map_bounds, LEVEL_TYPE_TRANSITION, make_boss_map, make_map, MAP_HEIGHT, MAP_WIDTH};
use crate::map::Map;
use graphics::render_functions::{initialize_fov, inventory_menu, menu, msgbox};
use crate::setup_game::main_menu;
use crate::tile::Tile;
use crate::util::transition::Transition;

mod event_processing{
    pub mod game_event_processing;
    pub mod audio_event_processor;
    pub mod game_occurrence;
    pub mod event_log_processor;
}
mod entity;
mod config {
    pub mod game_config;
}
mod map;
mod tile;
mod graphics{
    pub mod camera;
    pub mod render_functions;
}

mod game_engine;
mod framework;
mod setup_game;

mod audio {
    pub mod audio_engine;
}
mod util {
    pub mod transition;
    pub mod death_callback;
    pub mod namegen;
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


//TODO spilt xp store for player and drop xp into different values
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fighter {
    base_max_hp: i32,
    hp: i32,
    base_defense: i32,
    base_power: i32,
    xp: i32,
    on_death: DeathCallback
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Ai {
    Basic,
    Confused {                  // enum values can hold data. Dope
    previous_ai: Box<Ai>,
        num_turns: i32
    },
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

fn move_by(id: usize, dx: i32, dy: i32, map: &Map, entity: &mut [Entity]) {
    let (x,y) = entity[id].pos();
    if !is_blocked(x + dx, y + dy, map, entity) {
        entity[id].set_pos(x + dx, y + dy)
    }
}

fn player_move_or_attack(dx: i32, dy: i32, game: &mut GameEngine) {
    let x = game.entities[PLAYER].x + dx;
    let y = game.entities[PLAYER].y + dy;

    let map: &Map = game.map.borrow();
    let entities: &mut Vec<Entity> = game.entities.borrow_mut();
    let event_bus = game.event_bus.borrow_mut();

    let target_id = entities.iter().position(|entity| entity.fighter.is_some() && entity.pos() == (x,y));    // position() is an iterator function. It returns the position of the first to match the criteria
    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER, target_id, game.entities.borrow_mut());
            player.attack(target, event_bus);
            event_bus.add_event(GameEvent::from_type(EventType::PlayerAttack));
        }
        None => {
            move_by(PLAYER, dx, dy, map, entities);
            event_bus.add_event(GameEvent::from_type(EventType::PlayerMove));
        }
    }
}

fn pick_item_up(object_id: usize, game: &mut GameEngine) {
    if game.entities[PLAYER].inventory.len() >= 26 {
        game.messages.add(format!("Your pickets are full - you can't pickup the {}", game.entities[object_id].name), RED)
    } 
    else {
        let item = game.entities.swap_remove(object_id);
        game.messages.add(format!("You picked up the {}", item.name), GREEN);
        game.add_event(GameEvent::from_type_with_data(
            EventType::PlayerPickupItem,
            HashMap::from([("item".to_string(), EventData::String(item.name.clone()))])
        ));
        let index = game.entities[PLAYER].inventory.len();
        let slot = item.equipment.map(|e| e.slot);
        game.entities[PLAYER].inventory.push(item);

        // equip picked up item if it is equipment and the slot is open
        if let Some(slot) = slot {
            if get_equipped_id_in_slot(slot, &game.entities[PLAYER].inventory).is_none() {
                game.entities[PLAYER].inventory[index].equip(&mut game.messages);
            }
        }
    }
}

fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, entities: &mut [Entity]) {
    let dx = target_x - entities[id].x;
    let dy = target_y - entities[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();         // pythagorean path, causes mobs to get stuck on walls

    //normalize to length of 1, then round and convert to integer
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, entities);
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Equipment {
    slot: Slot,
    equipped: bool,
    max_hp_bonus: i32,
    power_bonus: i32,
    defense_bonus: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum Slot {
    LeftHand,
    RightHand,
    Head,
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Slot::LeftHand => write!(f, "left hand"),
            Slot::RightHand => write!(f, "right hand"),
            Slot::Head => write!(f, "head"),
        }
    }
}

fn from_dungeon_level(table: &[Transition], level: u32) -> u32 {
    table
        .iter()
        .rev()
        .find(|transition| level >= transition.level)
        .map_or(0, |transition| transition.value)
}

fn ai_take_turn(monster_id: usize, tcod: &Tcod, game: &mut GameEngine) {
    use Ai::*;
    if let Some(ai) = game.entities[monster_id].ai.take() {               // take() removes to the option from Option - it then becomes empty
        let new_ai = match ai {
            Basic => ai_basic(monster_id, tcod, game),
            Confused {
                previous_ai,
                num_turns   
            } => ai_confused(monster_id, tcod, game, previous_ai, num_turns)
        };
        game.entities[monster_id].ai = Some(new_ai);                      // the AI is then put back here
    }
}

fn ai_basic(monster_id: usize, tcod: &Tcod, game: &mut GameEngine) -> Ai {
    // a basic ai takes a turn. If you can see it, it can see you
    let entities: &mut Vec<Entity> = game.entities.borrow_mut();
    let event_bus = game.event_bus.borrow_mut();
    let (monster_x, monster_y) = entities[monster_id].pos();
    if tcod.fov.is_in_fov(monster_x, monster_y) {
        if entities[monster_id].distance_to(&entities[PLAYER]) >= 2.0 {
            // move towards player if far away
            let (player_x, player_y) = entities[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, &game.map, entities);
            event_bus.add_event(GameEvent::from_type(EventType::MonsterMove));

        } else {
            // close enough to start a war
            let (monster, player) = mut_two(monster_id, PLAYER, entities);
            monster.attack(player, event_bus);
            event_bus.add_event(GameEvent::from_type(EventType::MonsterAttack));

        }
    }
    Ai::Basic
}

fn ai_confused(monster_id:usize, _tcod: & Tcod, game: &mut GameEngine, previous_ai: Box<Ai>, num_turns: i32) -> Ai {
    let x = rand::thread_rng().gen_range(0, MAP_WIDTH);
    let y = rand::thread_rng().gen_range(0, MAP_HEIGHT);
    let messages = game.messages.borrow_mut();
    let map: &Map = game.map.borrow();
    let entities = game.entities.borrow_mut();
    move_towards(monster_id, x, y, map, entities);
    if num_turns == 0 {
        messages.add(format!("The {} is no longer confused", game.entities[monster_id].name), RED);
        *previous_ai
    } else {
        Ai::Confused{ previous_ai: previous_ai, num_turns: num_turns - 1}
    }
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

fn is_blocked(x: i32, y: i32, map: &Map, entity: &[Entity]) -> bool {
    if map[x as usize][y as usize].blocked {
        return true;
    }
    entity.
        iter()
        .any(|object| object.blocks && object.pos() == (x,y))
}

fn next_level(tcod: &mut Tcod, game: &mut GameEngine) {
    game.messages.add("You rest for a minute and recover your strength", VIOLET);
    let heal_hp = game.entities[PLAYER].max_hp() / 2;
    game.entities[PLAYER].heal(heal_hp);
    game.messages.add("You descend deeper into the dungeon ...", RED);
    game.dungeon_level += 1;
    let dungeon_level = game.dungeon_level;
    game.map = match from_dungeon_level(LEVEL_TYPE_TRANSITION, dungeon_level) {
        0 => make_map(game, dungeon_level),
        1 => make_boss_map(game, dungeon_level),
        _ => make_map(game, dungeon_level),
    };
    initialize_fov(tcod, &game.map);
    tcod.fov.compute_fov(game.entities[PLAYER].x, game.entities[PLAYER].y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO)
}

enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
}

fn use_item(inventory_id: usize, tcod: &mut Tcod, game: &mut GameEngine) {
    use Item::*;
    if let Some(item) = &game.entities[PLAYER].inventory[inventory_id].item {
        let on_use = match item {
            Heal => cast_heal,
            Lightning => cast_lightning,
            Confuse => cast_confuse,
            Fireball => cast_fireball,
            Artifact{name: _, value: _} => examine_artifact,
            Sword => toggle_equipment,
            Shield => toggle_equipment,
        };
        match on_use(inventory_id, tcod, game) {
            UseResult::UsedUp => {
                game.entities[PLAYER].inventory.remove(inventory_id);
            }
            UseResult::UsedAndKept => {}
            UseResult::Cancelled => {
                game.messages.add("Cancelled", WHITE);
            }
        }
    } else {
        game.messages.add(format!("The {} cannot be used.", game.entities[PLAYER].inventory[inventory_id].name), WHITE);
    }
}

fn drop_item(inventory_id: usize, game: &mut GameEngine) {
    //TODO dont default to players inventory
    let mut item = game.entities[PLAYER].inventory.remove(inventory_id);
    if item.equipment.is_some() {
        item.unequip(&mut game.messages);
    }
    item.set_pos(game.entities[PLAYER].x, game.entities[PLAYER].y);
    game.messages.add(format!("You dropped the {}.", item.name), YELLOW);
    game.entities.push(item);
}

fn cast_heal(
    _inventory_id: usize,
    _tcod: &mut Tcod,
    game: &mut GameEngine
) -> UseResult {
    let player = &mut game.entities[PLAYER];
    if let Some(fighter) = player.fighter {
        if fighter.hp == player.max_hp() {
            // game.messages.add("You're already at full health. ", RED);
            return UseResult::Cancelled;
        }
        // game.messages.add("Your wounds feel a bit better", LIGHT_VIOLET);
        player.heal(HEAL_AMOUNT);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

fn cast_lightning(
    _inventory_id: usize,
    tcod: &mut Tcod,
    game: &mut GameEngine,
) -> UseResult {

    let monster_id = closest_monster(tcod, game, LIGHTNING_RANGE);
    let entities: &mut Vec<Entity> = game.entities.borrow_mut();
    let event_bus = game.event_bus.borrow_mut();
    let messages = game.messages.borrow_mut();
    if let Some(monster_id) = monster_id {
        game.messages.add(
            format!("A lightning bolt strikes the {}! It deals {} points of damage.", entities[monster_id].name, LIGHTNING_DAMAGE),
            LIGHT_BLUE
        );
        if let Some(xp) = entities[monster_id].take_damage(LIGHTNING_DAMAGE, event_bus) {
            // TODO: determine attacker and award xp to them, not automatically to player
            entities[PLAYER].fighter.as_mut().unwrap().xp += xp;
        }
        UseResult::UsedUp
    } else {
        messages.add("No enemies are within range.", RED);
        UseResult::Cancelled
    }
}

fn cast_confuse(_inventory_id: usize, tcod: &mut Tcod, game: &mut GameEngine) -> UseResult {
    // let monster_id = target_monster(CONFUSE_RANGE, objects, tcod);
    let monster_id = target_monster(tcod, game, Some(CONFUSE_RANGE as f32));
    if let Some(monster_id) = monster_id {
        let old_ai = game.entities[monster_id].ai.take().unwrap_or(Ai::Basic);
        game.entities[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: CONFUSE_NUM_TURNS
        });
        game.messages.add(format!("The eyes of the {} glaze over, and it starts to stumble around.", game.entities[monster_id].name), LIGHT_GREEN);
        UseResult::UsedUp
    } else {
        game.messages.add("No enemy is close enough to strike", RED);
        UseResult::Cancelled
    }
}

fn cast_fireball(_inventory_id: usize, tcod: &mut Tcod, game: &mut GameEngine) -> UseResult {
    game.messages.add("Left-click a tile to cast a fireball at it; right-click or Esc to cancel", LIGHT_CYAN);
    let (x, y) = match target_tile(tcod, game, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled,
    };
    let entities: &mut Vec<Entity> = game.entities.borrow_mut();
    let event_bus = game.event_bus.borrow_mut();
    let messages = game.messages.borrow_mut();
    messages.add(format!("The fireball explodes, burning everything within {} tiles.", FIREBALL_RADIUS), ORANGE);
    let mut xp_to_gain = 0;
    for (id, obj) in entities.iter_mut().enumerate() {
        if obj.distance(x, y) <= FIREBALL_RADIUS as f32 && obj.fighter.is_some() {
            game.messages.add(format!("The {} gets burned for {} hit points.", obj.name, FIREBALL_DAMAGE), ORANGE);
            if let Some(xp) = obj.take_damage(FIREBALL_DAMAGE, event_bus) {
                if id != PLAYER {
                    xp_to_gain += xp;
                }
            }
        }
    }
    // TODO: determine attacker rather than awarding to player
    entities[PLAYER].fighter.as_mut().unwrap().xp += xp_to_gain;
    UseResult::UsedUp
}

fn examine_artifact(inventory_id: usize, _tcod: &mut Tcod, game: &mut GameEngine) -> UseResult {
    //TODO: dont default to player inventory
    match &game.entities[PLAYER].inventory[inventory_id].item {
        Some(item) => {
            match item {
                Item::Artifact {name, value} => {
                    game.messages.add(format!("This artifact is named {} and has a value of {}", name, value), GOLD);
                    return UseResult::UsedAndKept
                },
                _ => {
                    game.messages.add("Error: examine_artifact was called with an item that was not an artifact", DARK_RED);
                    return UseResult::Cancelled
                }
            };
        },
        None => {
            game.messages.add("Error: examine_artifact was called when there was no item", DARK_RED);
            return UseResult::Cancelled
        }
    };
}

fn toggle_equipment(inventory_id: usize, _tcod: &mut Tcod, game: &mut GameEngine) -> UseResult {
    //TODO: dont default to player inventory
    let messages = game.messages.borrow_mut();
    let player = game.entities[PLAYER].borrow_mut();
    let equipment = match player.inventory[inventory_id].equipment {
        Some(equipment) => equipment,
        None => return UseResult::Cancelled,
    };
    if let Some(current_equipment_id) = get_equipped_id_in_slot(equipment.slot, &player.inventory) {
        player.inventory[current_equipment_id].unequip(messages);
    }
    if equipment.equipped {
        player.inventory[inventory_id].unequip(messages);
    } else {
        player.inventory[inventory_id].equip(messages);
    }
    UseResult::UsedAndKept
}

fn get_equipped_id_in_slot(slot: Slot, inventory: &[Entity]) -> Option<usize> {
    for (inventory_id, item) in inventory.iter().enumerate() {
        if item.equipment.as_ref().map_or(false, |e| e.equipped && e.slot == slot) {
            return Some(inventory_id)
        }
    }
    None
}

fn closest_monster(tcod: &Tcod, game: &mut GameEngine, max_range: i32) -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range +1) as f32;

    for (id, object) in game.entities.iter().enumerate() {
        if id != PLAYER && object.fighter.is_some() && object.ai.is_some() && tcod.fov.is_in_fov(object.x, object.y) {
            let dist = game.entities[PLAYER].distance_to(object);
            if dist < closest_dist {
                closest_enemy = Some(id);
                closest_dist = dist;
            }
        }
    }
    closest_enemy
}

fn target_monster(
    tcod: &mut Tcod,
    game: &mut GameEngine,
    max_range: Option<f32>
) -> Option<usize> {
    loop {
        match target_tile(tcod, game, max_range) {
            Some((x,y)) =>
            for (id, obj) in game.entities.iter().enumerate() {
                if obj.pos() == (x, y) && obj.fighter.is_some() && id != PLAYER {
                    return Some(id)
                }
            },
            None => return None
        }
    }
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

#[derive(Serialize, Deserialize)]
pub struct Messages {
    messages: Vec<(String, Color)>
}

impl Messages {
    pub fn new() -> Self {
        Self {messages: vec![]}
    }

    pub fn add<T: Into<String>>(&mut self, message: T, color: Color) {
        self.messages.push((message.into(), color))
    }

    // returns an `impl Trait`. basically, it allows you to specify a return type without explicitly describing the type
    // The actual return type just needs to implement the trait specified.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &(String, Color)> {
        self.messages.iter()
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
