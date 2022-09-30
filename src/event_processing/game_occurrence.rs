use std::any::Any;
use std::borrow::Borrow;
use serde::{Deserialize, Serialize};
use tcod::colors::WHITE;
use crate::event_processing::game_event_processing::EventBusReader;
use crate::map::mapgen::{Map};
use crate::{EventData, EventProcessor, GameEvent, EventType};
use crate::entity::_entity::Entity;
use crate::map::tile::Tile;

#[derive(Serialize, Deserialize)]
pub struct GameOccurrenceEventProcessor {
    event_bus_reader: EventBusReader
}

impl GameOccurrenceEventProcessor {
    pub fn new() -> Self {
        GameOccurrenceEventProcessor {
            event_bus_reader: EventBusReader{head: 0}
        }
    }
}

#[typetag::serde]
impl EventProcessor for GameOccurrenceEventProcessor {
    fn process(&mut self, _map: &mut Map, entities: &mut Vec<Entity>, event_bus: &Vec<GameEvent>, max_events: usize, bus_tail: usize) {
        use EventType::*;
        if self.event_bus_reader.head != bus_tail {
            let event: &GameEvent = event_bus[self.event_bus_reader.head].borrow();
            match event.event_type {
                BossDie => {
                    let event_data = event.data.get("position");
                    match event_data {
                        Some(data) => match data {
                            EventData::TupleI32I32((x,y)) => {
                                let mut stairs = Entity::new(*x, *y - 1, '<', WHITE, "stairs", false);
                                stairs.always_visible = true;
                                entities.push(stairs);
                            },
                            _ => log::warn!("WARNING: attempted to pull position data, but it wasn't of the correct type")
                        }
                        _ => log::warn!("WARNING: attempted to pull position data, but it wasn't on the event")
                    }
                }
                _ => {}
            };
            self.event_bus_reader.head = (self.event_bus_reader.head + 1) % max_events;
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_id(&self) -> &str {
        "responder_event_processor"
    }
}

