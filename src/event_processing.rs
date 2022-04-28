use std::any::Any;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::Map;

#[derive(Serialize, Deserialize)]
pub enum EventType {
    PlayerAttack,
    PlayerMove,
    PlayerDie,
    MonsterAttack,
    MonsterDie,
    BossDie,
}

#[derive(Serialize, Deserialize)]
pub struct EventBusReader {
    pub head: usize,
}

#[derive(Serialize, Deserialize)]
pub enum EventData {
    Pos((i32, i32))
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
}

#[typetag::serde(tag = "type")]
pub trait EventProcessor {
    fn process(&mut self, map: &mut Map, event_bus: &Vec<GameEvent>, max_events: usize, bus_tail: usize);
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn as_any(&self) -> &dyn Any;
    fn get_id(&self) -> &str;
}