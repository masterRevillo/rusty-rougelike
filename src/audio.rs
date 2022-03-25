use soloud::*;

struct AudioSample{
    sample: audio::Wav,
    name: String
}

struct AudioEngine{
    samples: Vec<AudioSample>,
    bg: Option<audio::Wav>,
    player: Soloud
}

impl AudioEngine {
    pub fn new(player: Soloud) -> Self {
        AudioEngine {
            samples: vec![],
            bg: None,
            player: player
        }
    }

    pub fn set_bg(&mut self, bg: audio::Wav) {
        self.bg = Some(bg);
    }

    pub fn add_sample(&mut self, sample: AudioSample) {
        self.samples.push(sample);
    }

    pub fn play_sfx(&self, sfx_name: String) {
        let sample = self.samples.iter().find(|s| s.name.eq(&sfx_name));
        match sample {
            None => println!("no sample found with name {}", sfx_name),
            Some(s) => {
                self.player.play(&s.sample);
            }
        };
    }

    pub fn play_bg(&self) {
        match &self.bg {
            None => println!("no bg to play"),
            Some(bg) => {
                self.player.play(bg);
            }
        }
        
    }
}

fn init_sound_engine() -> Result<AudioEngine, Box<dyn std::error::Error>> {
    let mut audio_engine = AudioEngine::new(Soloud::default()?);

    let mut bg = audio::Wav::default();
    bg.load(&std::path::Path::new("house_lo.mp3"))?;
    
    let mut punch = audio::Wav::default();
    punch.load(&std::path::Path::new("punch.wav"))?;

    audio_engine.add_sample(AudioSample{sample: punch, name: "punch".to_string()});
    audio_engine.set_bg(bg);

    Ok(audio_engine)
}

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