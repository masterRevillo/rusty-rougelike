use std::any::Any;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::{Entity, Map};

#[derive(Serialize, Deserialize)]
pub enum EventType {
    PlayerAttack,
    EntityAttacked,
    PlayerMove,
    PlayerDie,
    MonsterAttack,
    MonsterMove,
    MonsterDie,
    BossDie,
    PlayerPickupItem
}

#[derive(Serialize, Deserialize)]
pub struct EventBus {
    pub bus: Vec<GameEvent>,
    pub bus_tail: usize,
    pub max_events: usize,
}

impl EventBus {
    pub fn add_event(&mut self, event: GameEvent) {
        if self.bus.len() > self.bus_tail {
            self.bus[self.bus_tail] = event;
        } else {
            self.bus.push(event);
        }
        self.bus_tail = (self.bus_tail + 1) % self.max_events;
    }
}

#[derive(Serialize, Deserialize)]
pub struct EventBusReader {
    pub head: usize,
}

#[derive(Serialize, Deserialize)]
pub enum EventData {
    TupleI32I32((i32, i32)),
    String(String),
    I32(i32),
    Boolean(bool)
}

#[derive(Serialize, Deserialize)]
pub struct GameEvent {
    pub event_type: EventType,
    pub data: HashMap<String, EventData>
}

impl GameEvent {
    pub fn from_type(event: EventType) -> Self {
        GameEvent {
            event_type: event,
            data: HashMap::new()
        }
    }

    pub fn from_type_with_data(event: EventType, data: HashMap<String, EventData>) -> Self {
        GameEvent {
            event_type: event,
            data
        }
    }

    pub fn get_data_as_flat_hashmap(&self) -> HashMap<&String, Box<dyn Display + '_>> {
        let mut map: HashMap<&String, Box<dyn Display>> = HashMap::new();
        for (key, value) in &self.data {
            match value {
                EventData::TupleI32I32((x,y)) => map.insert(&key, Box::new(format!("({},{})", x, y))),
                EventData::String(s) => map.insert(&key, Box::new(s)),
                EventData::I32(i) => map.insert(&key, Box::new(i)),
                EventData::Boolean(b) => map.insert(&key, Box::new(b))
            };
        }
        map
    }
}

#[typetag::serde(tag = "type")]
pub trait EventProcessor {
    fn process(&mut self, map: &mut Map, entities: &mut Vec<Entity>, event_bus: &Vec<GameEvent>, max_events: usize, bus_tail: usize);
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn as_any(&self) -> &dyn Any;
    fn get_id(&self) -> &str;
}
