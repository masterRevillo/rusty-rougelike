use std::collections::HashMap;
use bracket_lib::color::{DARK_RED, RGB};
use crate::{Entity, EventBus, EventData, EventType, GameEvent};
use serde::{Deserialize, Serialize};


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DeathCallback {
    Player,
    Monster,
    Boss
}


impl DeathCallback {
    pub fn callback(self, entity: &mut Entity, event_bus: &mut EventBus) {
        use DeathCallback::*;
        let callback = match self {
            Player => player_death,
            Monster => monster_death,
            Boss => boss_death,
        };
        callback(entity, event_bus);
    }
}

fn player_death(player: &mut Entity, event_bus: &mut EventBus) {
    // game.messages.add("You died!", RED);
    player.char = '%';
    player.color = RGB::from(DARK_RED);
    event_bus.add_event(GameEvent::from_type (EventType::PlayerDie));
}

fn monster_death(monster: &mut Entity, event_bus: &mut EventBus) {
    // game.messages.add(format!("{} died. It gives you {} xp.", monster.name, monster.fighter.unwrap().xp), ORANGE);
    monster.char = '%';
    monster.color = RGB::from(DARK_RED);
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
    event_bus.add_event(GameEvent::from_type(EventType::MonsterDie));
}

fn boss_death(monster: &mut Entity, event_bus: &mut EventBus) {
    // game.messages.add(format!("{} died. It gives you {} xp.", monster.name, monster.fighter.unwrap().xp), ORANGE);
    monster.char = '%';
    monster.color = RGB::from(DARK_RED);
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
    event_bus.add_event(GameEvent::from_type_with_data(
        EventType::BossDie,
        HashMap::from([("position".to_string(), EventData::TupleI32I32(monster.pos()))])
    ));
}