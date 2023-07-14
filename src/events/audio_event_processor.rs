use std::any::Any;

use serde::{Deserialize, Serialize};

use crate::{Entity, GameEvent, Map};
use crate::audio::audio_engine::AudioEngine;
use crate::events::game_event_processing::{EventBusReader, EventProcessor, EventType};

#[derive(Serialize, Deserialize)]
pub struct AudioEventProcessor {
    event_bus_reader: EventBusReader,
    #[serde(skip)]
    pub audio_engine: Option<AudioEngine>
}

impl AudioEventProcessor {
    pub fn new() -> Self {
        AudioEventProcessor {
            event_bus_reader: EventBusReader{head: 0},
            audio_engine: None
        }
    }

    pub fn set_audio_engine(&mut self, audio_engine: AudioEngine) {
        self.audio_engine = Some(audio_engine)
    }
}

#[typetag::serde]
impl EventProcessor for AudioEventProcessor {
    fn process(&mut self, _map: &mut Map, _entities: &mut Vec<Entity>, event_bus: &Vec<GameEvent>, max_events: usize, bus_tail: usize) {
        use EventType::*;
        if self.event_bus_reader.head != bus_tail {
            let sample_name = match event_bus[self.event_bus_reader.head].event_type {
                PlayerAttack => Some("punch".to_string()),
                MonsterAttack => Some("monster1".to_string()),
                MonsterDie => Some("monster_die1".to_string()),
                BossDie => Some("monster_die1".to_string()),
                PlayerPickupItem => Some("pick".to_string()),
                _ => None
            };
            if sample_name.is_some() {
                match &self.audio_engine {
                    Some(ae) => ae.play_sfx(sample_name.unwrap()),
                    None => log::warn!("Cannot play sound: Audio Engine not present")
                }
            }
            log::debug!("audio processing done - head: {}, tail:{}", self.event_bus_reader.head, bus_tail);
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
        "audio_event_processor"
    }
}

