use std::any::Any;
use std::borrow::Borrow;
use serde::{Deserialize, Serialize};
use crate::events::game_event_processing::EventBusReader;
use crate::{EventProcessor, GameEvent, Map, EventType, Entity};

#[derive(Serialize, Deserialize)]
pub struct EventLogProcessor {
    event_bus_reader: EventBusReader
}

impl EventLogProcessor {
    pub fn new() -> Self {
        EventLogProcessor {
            event_bus_reader: EventBusReader{head: 0}
        }
    }
}

#[typetag::serde]
impl EventProcessor for EventLogProcessor {
    fn process(&mut self, _map: &mut Map, _entities: &mut Vec<Entity>, event_bus: &Vec<GameEvent>, max_events: usize, bus_tail: usize) {
        use EventType::*;
        if self.event_bus_reader.head != bus_tail {
            let event: &GameEvent = event_bus[self.event_bus_reader.head].borrow();
            match event.event_type {
                EntityAttacked => {
                    let data_map = event.get_data_as_flat_hashmap();
                    let target_name = data_map.get(&"target_name".to_string()).unwrap();
                    let attacker_name = data_map.get(&"attacker_name".to_string()).unwrap();
                    let target_pos = data_map.get(&"target_pos".to_string()).unwrap();
                    let attacker_pos = data_map.get(&"attacker_pos".to_string()).unwrap();
                    let damage = data_map.get(&"damage".to_string()).unwrap();
                    log::info!("entity with name {} at {} attacked entity with name {} at {} for {} damage",
                        attacker_name, attacker_pos,
                        target_name, target_pos,
                        damage
                    );
                }
                _ => {}
            }
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
        "event_log_processor"
    }
}