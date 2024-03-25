use std::borrow::BorrowMut;

use serde::{Deserialize, Serialize};
// use tcod::{BackgroundFlag, Console, TextAlignment};
// use tcod::colors::{BLACK, DARKER_RED, LIGHT_GREEN, LIGHT_GREY, WHITE};
// use tcod::console::{blit, Root};
// use tcod::map::FovAlgorithm;

use crate::{AudioEventProcessor, Camera, Entity, EventBus, EventProcessor, GameConfig, GameEvent, in_map_bounds, MAP_HEIGHT, MAP_WIDTH, Messages, SCREEN_WIDTH, GameFramework};
use crate::save_game;
use crate::audio::audio_engine::AudioEngine;
use crate::graphics::render_functions::{BAR_WIDTH, display_menu, get_names_under_mouse, inventory_menu, INVENTORY_WIDTH, menu, MSG_HEIGHT, MSG_WIDTH, MSG_X, msgbox, PANEL_HEIGHT, PANEL_Y, render_bar, render_inventory_menu, render_level_up_menu};
use crate::map::mapgen::Map;
use crate::util::ai::ai_take_turn;

//fov settings
// pub const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
pub const FOV_LIGHT_WALLS: bool = true;
pub const TORCH_RADIUS: i32 = 10;

pub const PLAYER: usize = 0;

//parameters for leveling up
pub const LEVEL_UP_BASE: i32 = 200;
pub const LEVEL_UP_FACTOR: i32 = 150;
pub const LEVEL_SCREEN_WIDTH: i32 = 40;
pub const STATS_SCREEN_WIDTH: i32 = 30;

#[derive(Serialize, Deserialize)]
pub struct GameEngine {
    pub map: Map,
    pub messages: Messages,
    pub dungeon_level: u32,
    pub event_bus: EventBus,
    pub event_processors: Vec<Box<dyn EventProcessor>>,
    pub entities: Vec<Entity>,
    pub camera: Camera,
    #[serde(skip)]
    pub game_state: Box<GameState>
}

impl GameEngine {
    pub fn process_events(&mut self) {
        for processor in self.event_processors.iter_mut() {
            processor.as_mut().process(&mut self.map,&mut self.entities, &self.event_bus.bus, self.event_bus.max_events, self.event_bus.bus_tail);
        }
    }

    pub fn add_event(&mut self, event: GameEvent) {
        self.event_bus.add_event(event)
    }

    pub fn set_audio_engine(&mut self, configs: GameConfig) {
        let mut audio_engine = AudioEngine::new(configs).unwrap();
        audio_engine.load_samples();
        self.event_processors.iter_mut()
            .find(|p| p.get_id() == "audio_event_processor")
            .map(|p| if let Some(aep) = p.as_any_mut().downcast_mut::<AudioEventProcessor>() {
                aep.audio_engine = Some(audio_engine)
            });
    }

    pub fn play_background_music(&mut self) {
        self.event_processors.iter_mut()
            .find(|p| p.get_id() == "audio_event_processor")
            .map(|p| if let Some(aep) = p.as_any_mut().downcast_mut::<AudioEventProcessor>() {
                if let Some(ae) = aep.audio_engine.as_mut() {
                    ae.play_bg("ambient-metal".to_string());
                }
            });
    }

    pub fn render_all(&mut self, framework: &mut GameFramework, fov_recompute: bool) {
        let map: &mut Map = self.map.borrow_mut();
        let messages = self.messages.borrow_mut();
        let dungeon_level = self.dungeon_level;
        let camera = self.camera.borrow_mut();
        let player = & self.entities[PLAYER];
        camera.update(player);

        if fov_recompute {
            let player = &self.entities[PLAYER];
            // tcod.fov.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO)
        }
        let entities: &mut Vec<Entity> = self.entities.borrow_mut();
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let (x_in_camera, y_in_camera) = camera.get_pos_in_camera(x, y);
                if camera.in_bounds(x_in_camera, y_in_camera) && in_map_bounds(x, y) {
                    // let visible = framework.fov.is_in_fov(x, y);
                    let visible = true;
                    let color = match visible {
                        false => map[x as usize][y as usize].dark_color,
                        true => map[x as usize][y as usize].lit_color,
                    };
                    let surface_color = match visible {
                        false => map[x as usize][y as usize].surface_dark_color,
                        true => map[x as usize][y as usize].surface_lit_color,
                    };
                    let explored = &mut map[x as usize][y as usize].explored;
                    if visible {
                        *explored = true;
                    }
                    if *explored {
                        let c = map[x as usize][y as usize].surface_char;
                        // framework.con.set_default_foreground(surface_color);
                        // framework.con.put_char(x_in_camera, y_in_camera, c, BackgroundFlag::None);
                        // framework.con.set_char_background(x_in_camera, y_in_camera, color, BackgroundFlag::Set);
                    }
                }
            }
        }
        let mut to_draw: Vec<_> = entities
            .iter()
            .filter(|o|
                        // framework.fov.is_in_fov(o.x, o.y) ||
                            (o.always_visible && map[o.x as usize][o.y as usize].explored)  // is always visible and has been explored
            )
            .collect();
        to_draw.sort_by(|o1, o2|{o1.blocks.cmp(&o2.blocks)});
        for object in to_draw {
            object.draw(&mut framework.con, camera);
        }
        // reset GUI panel
        // framework.root.set_default_foreground(WHITE);
        // framework.panel.set_default_background(BLACK);
        // framework.panel.clear();
        // display player stats
        let hp = entities[PLAYER].fighter.map_or(0, |f| f.hp);
        let max_hp = entities[PLAYER].max_hp();
        render_bar(&mut framework.con, 1, 1, BAR_WIDTH, "HP", hp, max_hp, RGBA::from(LIGHT_GREEN), RGBA::from(DARK_RED));
        // get names at mouse location
        // framework.panel.set_default_foreground(LIGHT_GREY);
        // framework.panel.print_ex(1, 0, BackgroundFlag::None, TextAlignment::Left, get_names_under_mouse(framework.mouse, entities, &framework.fov));
        framework.con.print_color(1, 0, LIGHT_GRAY, BLACK, get_names_under_mouse(framework, entities));
        // display message log
        let mut y = MSG_HEIGHT as i32;
        for &(ref msg, color) in messages.iter().rev() {     // iterate through the messages in reverse order
            // let msg_height = framework.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
            let msg_height = 1;
            y -= msg_height;
            if y < 0 {
                break;
            }
            // framework.panel.set_default_foreground(color);
            // framework.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
            framework.con.draw_box(MSG_X, y, MSG_WIDTH, 0, RGBA::from(color), RGBA::from(BLACK));
            framework.con.print_color(MSG_X, y, RGBA::from(color), RGBA::from(BLACK), msg);
        }
        // display game level
        framework.con.print(
            1,
            3,
            format!("Level {}", dungeon_level)
        );
        // blit(
        //     &framework.panel,
        //     (0,0),
        //     (SCREEN_WIDTH, PANEL_HEIGHT),
        //     &mut framework.root,
        //     (0, PANEL_Y),
        //     1.0, 1.0
        // );
        // // blit the map
        // blit(
        //     &framework.con,
        //     (0, 0),
        //     (MAP_WIDTH, MAP_HEIGHT),
        //     &mut framework.root,
        //     (0, 0),
        //     1.0,
        //     1.0,
        // );
        (self.game_state.render)(framework, self)
    }


    pub fn run_game_loop(&mut self, framework: &mut GameFramework) {
        // for FOV recompute by setting player position to a weird value
        let mut previous_player_position = (-1, -1);

        while !framework.con.quitting {

            // match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            //     Some((_, Event::Mouse(m))) => framework.mouse = m,
            //     Some((_, Event::Key(k))) => framework.key = k,
            //     _ => framework.key = Default::default(),
            // }
            //
            let fov_recompute = previous_player_position != (self.entities[PLAYER].pos());

            self.render_all(framework, fov_recompute);

            // framework.root.flush();

            check_for_level_up(self);

            previous_player_position = self.entities[PLAYER].pos();
            let player_action = (self.game_state.handle_input)(framework, self);
            if player_action == PlayerAction::Exit {
                save_game(self).unwrap();
                break;
            }
            self.process_events();

            if self.entities[PLAYER].alive && player_action != DidntTakeTurn {
                for id in 0..self.entities.len() {
                    if self.entities[id].ai.is_some() {
                        ai_take_turn(id, &framework, self)
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

#[derive(Serialize, Deserialize)]
pub enum StateType {
    Main,
    UseFromInventory,
    DropFromInventory,
    ChoosingUpgrade,
    MainMenu
}

pub struct GameState {
    pub state_type: StateType,
    pub handle_input: &'static dyn Fn(&mut GameFramework, &mut GameEngine) -> PlayerAction,
    pub render: &'static dyn Fn(&mut GameFramework, &mut GameEngine)
}

impl GameState {
    pub fn main() -> Self {
        GameState {
            state_type: StateType::Main,
            handle_input: &handle_keys,
            render: &|_,_|()
        }
    }
    pub fn use_from_inventory() -> Self {
        GameState {
            state_type: StateType::UseFromInventory,
            handle_input: &handle_inventory_input,
            render: &|tcod,game| {
                render_inventory_menu(
                    tcod,
                    game,
                    "Select an item to use by pressing the matching key, or any other to \
                    cancel\n",
                );
            }
        }
    }

    pub fn drop_from_inventory() -> Self {
        GameState {
            state_type: StateType::ChoosingUpgrade,
            handle_input: &handle_inventory_input,
            render: &|tcod,game| {
                render_inventory_menu(
                    tcod,
                    game,
                    "Select an item to drop by pressing the matching key, or any other to \
                    cancel\n",
                );
            }
        }
    }


    pub fn choosing_upgrade() -> Self {
        GameState {
            state_type: StateType::DropFromInventory,
            handle_input: &handle_level_up_input,
            render: &|tcod,game| {
                render_level_up_menu(
                    tcod,
                    game,
                    "Level up! Choose a stat to increase: \n",
                );
            }
        }
    }
}

use std::borrow::Borrow;
use bracket_lib::color::{DARK_RED, LIGHT_GRAY, LIGHT_GREEN, RGBA};
use bracket_lib::prelude::VirtualKeyCode::Escape;
use bracket_lib::terminal::{BLACK, letter_to_option, VirtualKeyCode};
use crate::game_engine::PlayerAction::{DidntTakeTurn, TookTurn};
use crate::inventory::inventory_actions::{drop_item, use_item};

impl Default for GameState {
    fn default() -> Self {
        GameState::main()
    }
}

pub fn handle_keys(framework: &mut GameFramework, game: &mut GameEngine) -> PlayerAction {

    use crate::map::map_functions::next_level;
    use crate::entities::entity_actions::{pick_item_up, player_move_or_attack};
    use PlayerAction::*;
    use VirtualKeyCode::*;

    let player_alive = game.entities[PLAYER].alive;
    match framework.con.key {
        None => DidntTakeTurn,
        Some(key) => match (key, player_alive) {
            // (Enter, alt: true, ..}, _, _,) => {               // the 2 dots signify that we dont care about the other values of Key. Without them, the code wouldnt compile until all values were supplied
            //     let fullscreen = framework.root.is_fullscreen();
            //     framework.root.set_fullscreen(!fullscreen);
            //     DidntTakeTurn
            // },
            (Escape, _, ) => return Exit,

            // movement keys
            (Up, true) | (Numpad8, true) => {
                player_move_or_attack(0, -1, game);
                TookTurn
            },
            (Down, true) | (Numpad2, true) => {
                player_move_or_attack(0, 1, game);
                TookTurn
            },
            (Left, true) | (Numpad4, true) => {
                player_move_or_attack(-1, 0, game);
                TookTurn
            },
            (Right, true) | (Numpad6, true) => {
                player_move_or_attack(1, 0, game);
                TookTurn
            },
            (Home, true) | (Numpad7, true) => {
                player_move_or_attack(-1, -1, game);
                TookTurn
            },
            (PageUp, true) | (Numpad9, true) => {
                player_move_or_attack(1, -1, game);
                TookTurn
            },
            (End, true) | (Numpad1, true) => {
                player_move_or_attack(-1, 1, game);
                TookTurn
            },
            (PageDown, true) | (Numpad3, true) => {
                player_move_or_attack(1, 1, game);
                TookTurn
            },
            (Numpad5, true) | (Period, true) => {
                TookTurn
            },
            (G, true) => {
                let item_id = game.entities.iter().position(|object| object.pos() == game.entities[PLAYER].pos() && object.item.is_some());
                if let Some(item_id) = item_id {
                    pick_item_up(item_id, game);
                }
                DidntTakeTurn
            },
            (I, true) => {
                log::info!("Changing game state to use from inventory");
                game.game_state = Box::new(GameState::use_from_inventory());
                DidntTakeTurn
            },
            (D, true) => {
                log::info!("Changing game state to drop from inventory");
                game.game_state = Box::new(GameState::drop_from_inventory());
                DidntTakeTurn
            },
            (Comma, true) if framework.con.shift => {
                let player_on_stairs = game.entities
                    .iter()
                    .any(|object| object.pos() == game.entities[PLAYER].pos() && object.name == "stairs");
                if player_on_stairs {
                    next_level(framework, game);
                }
                DidntTakeTurn
            },
            (C, true) => {
                let player = &game.entities[PLAYER];
                let level = player.level;
                let level_up_xp = LEVEL_UP_BASE + level * LEVEL_UP_FACTOR;
                if let Some(fighter) = player.fighter.as_ref() {
                    let msg = format!(
                        "Player stats: \n Level: {}\nExperience: {}\nExperience to level up: {}\n\nMaximum HP: {}\nAttack: {}\nbase_Defense: {}",
                        level, fighter.xp, level_up_xp, player.max_hp(), player.power(), player.defense()
                    );
                    msgbox(&msg, STATS_SCREEN_WIDTH, &mut framework.con);
                }
                DidntTakeTurn
            },
            _ => DidntTakeTurn // everything else
        }
    }
}

fn handle_inventory_input(
    framework: &mut GameFramework,
    game: &mut GameEngine,
) -> PlayerAction {
    use VirtualKeyCode::*;
    use PlayerAction::*;

    match framework.con.key {
        None => DidntTakeTurn,
        Some(key) => match key {
            // Enter => {
                // let fullscreen = framework.root.is_fullscreen();
                // framework.root.set_fullscreen(!fullscreen);
                // DidntTakeTurn
            // },
            Escape => {
                log::info!("Changing game state to main");
                game.game_state = Box::new(GameState::main());
                DidntTakeTurn
            },
            _ => {
                let selection = letter_to_option(key);
                if selection > -1 && selection < game.entities[PLAYER].inventory.len() as i32 {
                    return handle_inventory(
                        key,
                        framework,
                        game,
                        match game.game_state.state_type {
                            StateType::UseFromInventory => &use_item,
                            _ => &drop_item
                        })
                }
                DidntTakeTurn
            }
        }
    }
}


fn handle_level_up_input(
    framework: &mut GameFramework,
    game: &mut GameEngine,
) -> PlayerAction {
    use VirtualKeyCode::*;
    use PlayerAction::*;

    match framework.con.key {
        None => DidntTakeTurn,
        Some(key) => match key {
            // (Enter, alt: true, ..}, _, ) => {
            //     let fullscreen = tcod.root.is_fullscreen();
            //     tcod.root.set_fullscreen(!fullscreen);
            //     DidntTakeTurn
            // },
            Escape => {
                log::info!("Changing game state to main");
                game.game_state = Box::new(GameState::main());
                DidntTakeTurn
            },
            _ => {
                return handle_level_up_selection(
                    key,
                    game,
                )
            },
        }
    }
}


fn handle_inventory(
    key: VirtualKeyCode,
    framework: &mut GameFramework,
    game: &mut GameEngine,
    inventory_action: &'static dyn Fn(usize, &mut GameFramework, &mut GameEngine)
) -> PlayerAction {
    let inventory = &game.entities[PLAYER].inventory;
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
    } else {
        inventory.iter().map(|item| {
            match item.equipment {
                Some(equipment) if equipment.equipped => {
                    format!("{} (on {})", item.name, equipment.slot)
                }
                _ => item.name.clone()
            }

        }).collect()
    };
    let selection = letter_to_option(key);
    if selection > -1 && selection < inventory.len() as i32 {
        let index = selection as usize;
        if index < options.len() {
            inventory_action(index, framework, game);
            log::info!("Changing game state to main");
            game.game_state = Box::new(GameState::main());
            TookTurn
        } else {
            DidntTakeTurn
        }
    } else {
        DidntTakeTurn
    }
}

fn check_for_level_up(game: &mut GameEngine) {
    let player = &mut game.entities[PLAYER];
    let level_up_xp = LEVEL_UP_BASE + LEVEL_UP_FACTOR * player.level;
    if player.fighter.as_ref().map_or(0, |f| f.xp) >= level_up_xp {
        player.level += 1;
        game.game_state = Box::new(GameState::choosing_upgrade());
        log::info!("Changing game state to Leveling Up")
    }
}


fn handle_level_up_selection(
    key: VirtualKeyCode,
    game: &mut GameEngine,
) -> PlayerAction {
    let player = &mut game.entities[PLAYER];
    let fighter = player.fighter.as_mut().unwrap();
    let level_up_xp = LEVEL_UP_BASE + LEVEL_UP_FACTOR * player.level;
    let selection = letter_to_option(key);
    // TODO: dont hardcode to 3 - somehow determine number of upgrade choices
    if selection < 3 {
        match selection as usize {
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
        fighter.xp -= level_up_xp;
        log::info!("Changing game state to main");
        game.game_state = Box::new(GameState::main());
        DidntTakeTurn
    } else {
        DidntTakeTurn
    }
}

