use std::any::Any;
use soloud::*;
use crate::event_processing::{EventBusReader, EventProcessor, EventType};
use crate::{GameEvent, Map};
use serde::{Deserialize, Serialize};

pub struct AudioSample {
    sample: audio::Wav,
    name: String
}

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

#[derive(Serialize, Deserialize)]
pub struct AudioEngine {
    #[serde(skip)]
    samples: Vec<AudioSample>,
    #[serde(skip)]
    bg: Option<audio::Wav>,
    #[serde(skip)]
    player: Option<Soloud>,
}

#[typetag::serde]
impl EventProcessor for AudioEventProcessor {
    fn process(&mut self, _map: &mut Map, event_bus: &Vec<GameEvent>, max_events: usize, bus_tail: usize) {
        use EventType::*;
        if self.event_bus_reader.head != bus_tail {
            let sample_name = match event_bus[self.event_bus_reader.head].event_type {
                PlayerAttack => Some("punch".to_string()),
                MonsterAttack => Some("monster1".to_string()),
                MonsterDie => Some("monster_die1".to_string()),
                BossDie => Some("monster_die1".to_string()),
                _ => None
            };
            if sample_name.is_some() {
                match &self.audio_engine {
                    Some(ae) => ae.play_sfx(sample_name.unwrap()),
                    None => println!("Cannot play sound: Audio Engine not present")
                }
            }
            println!("audio processing done - head: {}, tail:{}", self.event_bus_reader.head, bus_tail);
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

impl AudioEngine {
    pub fn new() -> Result<AudioEngine, Box<dyn std::error::Error>> {
        let engine = AudioEngine {
            samples: vec![],
            bg: None,
            player: Some(Soloud::default()?),
        };
        Ok(engine)
    }

    pub fn load_samples(&mut self) {
        let mut punch = audio::Wav::default();
        punch.load(&std::path::Path::new("punch.wav"));
        self.add_sample(AudioSample{sample: punch, name: "punch".to_string()});

        let mut monster1 = audio::Wav::default();
        monster1.load(&std::path::Path::new("monster1.wav"));
        self.add_sample(AudioSample{sample: monster1, name: "monster1".to_string()});

        let mut monster_die1 = audio::Wav::default();
        monster_die1.load(&std::path::Path::new("monster_die1.mp3"));
        self.add_sample(AudioSample{sample: monster_die1, name: "monster_die1".to_string()});


        let mut bg = audio::Wav::default();
        bg.load(&std::path::Path::new("ambient-metal.wav"));

        self.set_bg(bg);
    }

    pub fn set_bg(&mut self, bg: audio::Wav) {
        self.bg = Some(bg);
    }

    pub fn add_sample(&mut self, sample: AudioSample) {
        self.samples.push(sample);
    }

    pub fn play_sfx(&self, sfx_name: String) {
        let sample = self.samples.iter().find(|s| s.name.eq(&sfx_name));
        println!("playing sample with name {}", sfx_name);
        match sample {
            None => println!("no sample found with name {}", sfx_name),
            Some(s) => {
                if let Some(pl) = self.player.as_ref() {
                    let _handle = pl.play_ex(
                        &s.sample,
                        -1.0, 0.0, false, Handle::PRIMARY
                    );
                }
            }
        };
    }

    pub fn play_bg(&mut self) {
        match &self.bg {
            None => println!("no bg to play"),
            Some(bg) => {
                if let Some(pl) = self.player.as_mut() {
                    println!("about to play bg music");
                    let handle = pl.play_background_ex(
                        bg,
                        -1.0, false, Handle::PRIMARY
                    );
                    pl.set_looping(handle, true);
                }
            }
        }
        
    }
}

// pub fn init_sound_engine() -> Result<AudioEngine, Box<dyn std::error::Error>> {
//     let mut audio_engine = AudioEngine::new(Soloud::default()?);
//
//     let mut bg = audio::Wav::default();
//     bg.load(&std::path::Path::new("house_lo.mp3"))?;
//
//     let mut punch = audio::Wav::default();
//     punch.load(&std::path::Path::new("punch.wav"))?;
//
//     audio_engine.add_sample(AudioSample{sample: punch, name: "punch".to_string()});
//     audio_engine.set_bg(bg);
//
//     Ok(audio_engine)
// }

// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let audio_engine = init_sound_engine().unwrap();

//     audio_engine.play_bg();

//     for _ in 0..10 {
//         audio_engine.play_sfx("punch".to_string());
//         std::thread::sleep(std::time::Duration::from_millis(300));
//     }

//     std::thread::sleep(std::time::Duration::from_millis(3000));

//     Ok(())
// }