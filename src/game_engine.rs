use std::borrow::BorrowMut;

use serde::{Deserialize, Serialize};
use tcod::{BackgroundFlag, Console, TextAlignment};
use tcod::colors::{BLACK, DARKER_RED, LIGHT_GREEN, LIGHT_GREY, WHITE};
use tcod::console::blit;
use tcod::map::FovAlgorithm;

use crate::{AudioEventProcessor, Camera, Entity, EventBus, EventProcessor, GameConfig, GameEvent, in_map_bounds, MAP_HEIGHT, MAP_WIDTH, Messages, SCREEN_WIDTH, Tcod};
use crate::save_game;
use crate::audio::audio_engine::AudioEngine;
use crate::graphics::render_functions::{BAR_WIDTH, get_names_under_mouse, inventory_menu, menu, MSG_HEIGHT, MSG_WIDTH, MSG_X, msgbox, PANEL_HEIGHT, PANEL_Y, render_bar};
use crate::map::mapgen::Map;
use crate::util::ai::ai_take_turn;

//fov settings
pub const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
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
    pub input_handler: Box<InputHandler>
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

    pub fn render_all(&mut self, tcod: &mut Tcod, fov_recompute: bool) {
        let map: &mut Map = self.map.borrow_mut();
        let messages = self.messages.borrow_mut();
        let dungeon_level = self.dungeon_level;
        let camera = self.camera.borrow_mut();
        let player = & self.entities[PLAYER];
        camera.update(player);

        if fov_recompute {
            let player = &self.entities[PLAYER];
            tcod.fov.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO)
        }
        let entities: &mut Vec<Entity> = self.entities.borrow_mut();
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let (x_in_camera, y_in_camera) = camera.get_pos_in_camera(x, y);
                if camera.in_bounds(x_in_camera, y_in_camera) && in_map_bounds(x, y) {
                    let visible = tcod.fov.is_in_fov(x, y);
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
                        tcod.con.set_default_foreground(surface_color);
                        tcod.con.put_char(x_in_camera, y_in_camera, c, BackgroundFlag::None);
                        tcod.con.set_char_background(x_in_camera, y_in_camera, color, BackgroundFlag::Set);
                    }
                }
            }
        }
        let mut to_draw: Vec<_> = entities
            .iter()
            .filter(|o|
                        tcod.fov.is_in_fov(o.x, o.y)                                            // is in fov
                            || (o.always_visible && map[o.x as usize][o.y as usize].explored)  // is always visible and has been explored
            )
            .collect();
        to_draw.sort_by(|o1, o2|{o1.blocks.cmp(&o2.blocks)});
        for object in to_draw {
            object.draw(&mut tcod.con, camera);
        }
        // reset GUI panel
        tcod.root.set_default_foreground(WHITE);
        tcod.panel.set_default_background(BLACK);
        tcod.panel.clear();
        // display player stats
        let hp = entities[PLAYER].fighter.map_or(0, |f| f.hp);
        let max_hp = entities[PLAYER].max_hp();
        render_bar(&mut tcod.panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, LIGHT_GREEN, DARKER_RED);
        // get names at mouse location
        tcod.panel.set_default_foreground(LIGHT_GREY);
        tcod.panel.print_ex(1, 0, BackgroundFlag::None, TextAlignment::Left, get_names_under_mouse(tcod.mouse, entities, &tcod.fov));
        // display message log
        let mut y = MSG_HEIGHT as i32;
        for &(ref msg, color) in messages.iter().rev() {     // iterate through the messages in reverse order
            let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
            y -= msg_height;
            if y < 0 {
                break;
            }
            tcod.panel.set_default_foreground(color);
            tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        }
        // display game level
        tcod.panel.print_ex(1, 3, BackgroundFlag::None, TextAlignment::Left, format!("Level {}", dungeon_level));
        blit(
            &tcod.panel,
            (0,0),
            (SCREEN_WIDTH, PANEL_HEIGHT),
            &mut tcod.root,
            (0, PANEL_Y),
            1.0, 1.0
        );
        // blit the map
        blit(
            &tcod.con,
            (0, 0),
            (MAP_WIDTH, MAP_HEIGHT),
            &mut tcod.root,
            (0, 0),
            1.0,
            1.0,
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

#[derive(Serialize, Deserialize)]
pub enum HandlerType {
    Main
}

pub struct InputHandler {
    pub handler_type: HandlerType,
    pub handle_input: &'static dyn Fn(&mut Tcod, &mut GameEngine) -> PlayerAction
}

impl InputHandler {
    pub fn main() -> Self {
        InputHandler {
            handler_type: HandlerType::Main,
            handle_input: &handle_keys
        }
    }
}

use std::borrow::Borrow;
impl Default for InputHandler {
    fn default() -> Self {
        InputHandler {
            handler_type: HandlerType::Main,
            handle_input: &{ | _: &mut _, _:&mut _ | PlayerAction::DidntTakeTurn }
        }
    }
}

pub fn handle_keys(tcod: &mut Tcod, game: &mut GameEngine) -> PlayerAction {
    use tcod::input::KeyCode::*;
    use tcod::input::Key;
    use crate::map::map_functions::next_level;
    use crate::inventory::inventory_actions::{drop_item, use_item};
    use crate::entities::entity_actions::{pick_item_up, player_move_or_attack};
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

pub fn run_game_loop(tcod: &mut Tcod, game: &mut GameEngine) {
    use tcod::input::{self, Event};
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
        let player_action = (game.input_handler.handle_input)(tcod, game);
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

