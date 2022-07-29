use std::any::Any;
use soloud::*;
use serde::{Deserialize, Serialize};
use crate::config::game_config::GameConfig;
use crate::{Entity, GameEvent, Map};


pub struct AudioSample {
    sample: audio::Wav,
    name: String
}

#[derive(Serialize, Deserialize)]
pub struct AudioEngine {
    #[serde(skip)]
    samples: Vec<AudioSample>,
    #[serde(skip)]
    bg: Option<audio::Wav>,
    #[serde(skip)]
    player: Option<Soloud>,
    configs: Box<GameConfig>
}

impl AudioEngine {
    pub fn new(configs: GameConfig) -> Result<AudioEngine, Box<dyn std::error::Error>> {
        let engine = AudioEngine {
            samples: vec![],
            bg: None,
            player: Some(Soloud::default()?),
            configs: Box::new(configs)
        };
        Ok(engine)
    }

    pub fn load_samples(&mut self) {
        let mut punch = audio::Wav::default();
        punch.load(&std::path::Path::new("assets/audio/punch.wav"));
        self.add_sample(AudioSample{sample: punch, name: "punch".to_string()});

        let mut monster1 = audio::Wav::default();
        monster1.load(&std::path::Path::new("assets/audio/monster1.wav"));
        self.add_sample(AudioSample{sample: monster1, name: "monster1".to_string()});

        let mut monster_die1 = audio::Wav::default();
        monster_die1.load(&std::path::Path::new("assets/audio/monster_die1.mp3"));
        self.add_sample(AudioSample{sample: monster_die1, name: "monster_die1".to_string()});

        let mut pick = audio::Wav::default();
        pick.load(&std::path::Path::new("assets/audio/pick.wav"));
        self.add_sample(AudioSample{sample: pick, name: "pick".to_string()});

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
        if !self.configs.play_sfx {return;}
        let sample = self.samples.iter().find(|s| s.name.eq(&sfx_name));
        log::debug!("playing sample with name {}", sfx_name);
        match sample {
            None => log::warn!("no sample found with name {}", sfx_name),
            Some(s) => {
                if let Some(pl) = self.player.as_ref() {
                    let _handle = pl.play_ex(
                        &s.sample,
                        self.configs.sfx_volume, 0.0, false, Handle::PRIMARY
                    );
                }
            }
        };
    }

    pub fn play_bg(&mut self) {
        if !self.configs.play_bgm {return;}
        match &self.bg {
            None => log::warn!("no bgm to play"),
            Some(bg) => {
                if let Some(pl) = self.player.as_mut() {
                    log::debug!("about to play bg music");
                    let handle = pl.play_background_ex(
                        bg,
                        self.configs.bgm_volume, false, Handle::PRIMARY
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