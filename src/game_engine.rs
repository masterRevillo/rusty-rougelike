use std::borrow::{Borrow, BorrowMut};
use tcod::{BackgroundFlag, Console, TextAlignment};
use tcod::colors::{BLACK, DARKER_RED, LIGHT_GREEN, LIGHT_GREY, WHITE};
use tcod::console::blit;
use crate::{AudioEventProcessor, BAR_WIDTH, Camera, Entity, EventBus, EventProcessor, FOV_ALGO, FOV_LIGHT_WALLS, GameConfig, GameEvent, in_map_bounds, MAP_HEIGHT, MAP_WIDTH, Messages, MSG_HEIGHT, MSG_WIDTH, MSG_X, PANEL_HEIGHT, PANEL_Y, PLAYER, SCREEN_WIDTH, Tcod, TORCH_RADIUS};
use crate::map::Map;
use crate::audio::audio_engine::AudioEngine;
use serde::{Deserialize, Serialize};
use crate::graphics::render_functions::{get_names_under_mouse, render_bar};

#[derive(Serialize, Deserialize)]
pub struct GameEngine {
    pub map: Map,
    pub messages: Messages,
    //TODO: move inventory out of the game struct
    pub dungeon_level: u32,
    pub event_bus: EventBus,
    pub event_processors: Vec<Box<dyn EventProcessor>>,
    pub entities: Vec<Entity>,
    pub camera: Camera
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
        let player = self.entities[PLAYER].borrow();
        camera.update(player);

        if fov_recompute {
            let player = self.entities[PLAYER].borrow();
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