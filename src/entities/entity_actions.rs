use std::borrow::BorrowMut;
use std::collections::HashMap;

use tcod::colors::{GREEN, RED};

use crate::{MAP_HEIGHT, MAP_WIDTH};
use crate::entities::entity::Entity;
use crate::events::game_event_processing::{EventData, EventType, GameEvent};
use crate::framework::Tcod;
use crate::game_engine::{GameEngine, PLAYER};
use crate::inventory::inventory_actions::get_equipped_id_in_slot;
use crate::map::map_functions::is_blocked;
use crate::map::mapgen::Map;
use crate::util::mut_two::mut_two;

pub fn move_by(id: usize, dx: i32, dy: i32, map: &Map, entity: &mut [Entity]) {
    let (x,y) = entity[id].pos();
    if !is_blocked(x + dx, y + dy, map, entity) {
        entity[id].set_pos(x + dx, y + dy)
    }
}

pub fn player_move_or_attack(dx: i32, dy: i32, game: &mut GameEngine) {
    let x = game.entities[PLAYER].x + dx;
    let y = game.entities[PLAYER].y + dy;

    let map: &Map = &game.map;
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

pub fn pick_item_up(object_id: usize, game: &mut GameEngine) {
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

pub fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, entities: &mut [Entity]) {
    let dx = target_x - entities[id].x;
    let dy = target_y - entities[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();         // pythagorean path, causes mobs to get stuck on walls

    //normalize to length of 1, then round and convert to integer
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, entities);
}

pub fn target_tile(
    tcod: &mut Tcod,
    game: &mut GameEngine,
    max_range: Option<f32>
) -> Option<(i32, i32)> {
    use tcod::input::KeyCode::Escape;
    use tcod::input::{self, Event};
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
