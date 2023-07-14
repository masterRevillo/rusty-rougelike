use std::collections::HashMap;
use soloud::*;
use serde::{Deserialize, Serialize};
use crate::config::game_config::GameConfig;


pub struct PlayableAudio {
    audio: audio::Wav,
    name: String
}

#[derive(Serialize, Deserialize)]
pub struct AudioEngine {
    #[serde(skip)]
    samples: Vec<PlayableAudio>,
    #[serde(skip)]
    bgm: Vec<PlayableAudio>,
    #[serde(skip)]
    player: Option<Soloud>,
    configs: Box<GameConfig>,
    base_file_path: &'static str,
    sample_file_names: HashMap<&'static str, &'static str>,
    bgm_file_names: HashMap<&'static str, &'static str>
}


impl AudioEngine {
    pub fn new(configs: GameConfig) -> Result<AudioEngine, Box<dyn std::error::Error>> {
        let engine = AudioEngine {
            samples: vec![],
            bgm: vec![],
            player: Some(Soloud::default()?),
            configs: Box::new(configs),
            base_file_path: "assets/audio/",
            sample_file_names: HashMap::from([
                ("punch", "punch.wav"),
                ("monster1", "monster1.wav"),
                ("monster_die1", "monster_die1.mp3"),
                ("pick", "pick.wav"),
            ]),
            bgm_file_names: HashMap::from([
                ("ambient-metal", "ambient-metal.wav"),
            ]),
        };
        Ok(engine)
    }

    pub fn load_samples(&mut self) {
        let samples = &mut self.samples;
        let sample_file_names = &self.sample_file_names;
        let bgm = &mut self.bgm;
        let bgm_file_names = &self.bgm_file_names;
        let base_file_path = &self.base_file_path.to_string();

        sample_file_names.iter().for_each(|e| {
                let mut sample = audio::Wav::default();
                let _ = sample.load(&std::path::Path::new((base_file_path.to_owned() + e.1).as_str()));
                samples.push(PlayableAudio{audio: sample, name: String::from(e.0.to_string())});
            }
        );

        bgm_file_names.iter().for_each(|e|{
            let mut audio = audio::Wav::default();
            let _ = audio.load(&std::path::Path::new((base_file_path.to_owned() + e.1).as_str()));
            bgm.push(PlayableAudio{audio, name: String::from(e.0.to_string())});
        });
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
                        &s.audio,
                        self.configs.sfx_volume, 0.0, false, Handle::PRIMARY
                    );
                }
            }
        };
    }

    pub fn play_bg(&mut self, bgm_name: String) {
        if !self.configs.play_bgm {return;}
        let audio = self.bgm.iter().find(|a| a.name.eq(&bgm_name));
        match audio {
            None => log::warn!("no bgm to play"),
            Some(bg) => {
                if let Some(pl) = self.player.as_mut() {
                    log::debug!("about to play bg music");
                    let handle = pl.play_background_ex(
                        &bg.audio,
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