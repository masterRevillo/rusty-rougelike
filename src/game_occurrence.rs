use std::any::Any;
use std::borrow::Borrow;
use serde::{Deserialize, Serialize};
use crate::event_processing::EventBusReader;
use crate::{EventProcessor, EventType, EventData, GameEvent, Map, Tile};

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
    fn process(&mut self, map: &mut Map, event_bus: &Vec<GameEvent>, max_events: usize, bus_tail: usize) {
        use EventType::*;
        if self.event_bus_reader.head != bus_tail {
            let event: &GameEvent = event_bus[self.event_bus_reader.head].borrow();
            match event.event_type {
                BossDie => {
                    let event_data = event.data.get("position");
                    match event_data {
                        Some(data) => match data {
                            EventData::Pos((x,y)) => map[*x as usize][*y as usize] = Tile::wall(),
                            _ => println!("WARNING: attempted to pull position data, but it wasn't of the correct type")
                        }
                        _ => println!("WARNING: attempted to pull position data, but it wasn't on the event")
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

