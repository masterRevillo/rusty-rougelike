use std::borrow::BorrowMut;

use rand::Rng;
use serde::{Deserialize, Serialize};
use tcod::colors::RED;

use crate::entities::entity::Entity;
use crate::entities::entity_actions::move_towards;
use crate::events::game_event_processing::{EventType, GameEvent};
use crate::framework::Tcod;
use crate::game_engine::{GameEngine, PLAYER};
use crate::map::mapgen::{Map, MAP_HEIGHT, MAP_WIDTH};
use crate::util::mut_two::mut_two;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Ai {
    Basic,
    Confused {                  // enum values can hold data. Dope
    previous_ai: Box<Ai>,
        num_turns: i32
    },
}

pub fn ai_take_turn(monster_id: usize, tcod: &Tcod, game: &mut GameEngine) {
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
    let map: &Map = &game.map;
    let entities = game.entities.borrow_mut();
    move_towards(monster_id, x, y, map, entities);
    if num_turns == 0 {
        messages.add(format!("The {} is no longer confused", game.entities[monster_id].name), RED);
        *previous_ai
    } else {
        Ai::Confused{ previous_ai: previous_ai, num_turns: num_turns - 1}
    }
}