use std::borrow::BorrowMut;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use tcod::{BackgroundFlag, Console, TextAlignment};
use tcod::colors::{DARK_RED, RED, SKY, WHITE};
use crate::{AudioEventProcessor, Camera, Entity, Equipment, EventBus, EventLogProcessor, Fighter, GameEngine, GameOccurrenceEventProcessor, initialize_fov, Item, load_configs, make_map, MAP_HEIGHT, MAP_WIDTH, menu, Messages, msgbox, PLAYER, run_game_loop, SCREEN_HEIGHT, SCREEN_WIDTH, Slot, Tcod};
use crate::util::death_callback::DeathCallback;

pub fn main_menu(tcod: &mut Tcod) {
    let img = tcod::image::Image::from_file("desert.png").ok().expect("Background image not found");

    while !tcod.root.window_closed() {
        tcod::image::blit_2x(&img, (1800,800), (-1,-1), &mut tcod.root, (0,0));

        tcod.root.set_default_foreground(DARK_RED);
        tcod.root.print_ex(SCREEN_WIDTH / 2, SCREEN_HEIGHT / 2 - 6, BackgroundFlag::None, TextAlignment::Center, "THE HALLS OF RUZT");
        tcod.root.print_ex(SCREEN_WIDTH / 2, SCREEN_HEIGHT / 2 - 4, BackgroundFlag::None, TextAlignment::Center, "By Rev");

        let choices = &["Play a new game", "Continue last game", "Quit"];
        let choice = menu("", choices, 24, &mut tcod.root);

        match choice {
            Some(0) => {
                let mut game= new_game(tcod);
                run_game_loop(tcod, &mut game);
            }
            Some(1) => {
                match load_game() {
                    Ok(mut game) => {
                        initialize_fov(tcod, &game.map);
                        run_game_loop(tcod, &mut game);
                    },
                    Err(_e) => {
                        msgbox("\n No saved game to load.\n", 24, &mut tcod.root);
                        continue;
                    }
                }
            }
            Some(2) => {
                break;
            },
            _=> {}
        }
    }
}

pub fn load_game() -> Result<GameEngine, Box<dyn Error>> {
    let config = load_configs();
    let mut json_save_state = String::new();
    let mut file = File::open("savegame")?;
    file.read_to_string(&mut json_save_state)?;
    let mut result = serde_json::from_str::<GameEngine>(&json_save_state)?;
    result.set_audio_engine(config);
    result.play_background_music();
    Ok(result)
}

pub fn new_game(tcod: &mut Tcod) -> GameEngine {
    let config = load_configs();
    let mut player = Entity::new(0, 0, '@', WHITE, "player", true);
    player.alive = true;
    player.fighter = Some(Fighter {
        base_max_hp: 30,
        hp: 30,
        base_defense: 2,
        base_power: 3,
        xp: 200,
        on_death: DeathCallback::Player
    });

    let mut entities = vec![player];

    let mut game = GameEngine {
        map: vec![vec![]],
        messages: Messages::new(),
        dungeon_level: 1,
        event_bus: EventBus {
            bus: vec![],
            bus_tail: 0,
            max_events: 32,
        },
        event_processors: vec![
            Box::new(AudioEventProcessor::new()),
            Box::new(GameOccurrenceEventProcessor::new()),
            Box::new(EventLogProcessor::new())
        ],
        entities,
        camera: Camera{
            x: 0, y: 0,
            width: SCREEN_WIDTH, height: SCREEN_HEIGHT,
            map_width: MAP_WIDTH, map_height: MAP_HEIGHT
        }
    };
    let map = make_map(game.borrow_mut(), 1);
    game.map = map;


    game.set_audio_engine(config);
    game.play_background_music();

    let mut dagger = Entity::new(0, 0, '-', SKY, "dagger", false);
    dagger.item = Some(Item::Sword);
    dagger.equipment = Some(Equipment {
        equipped: true, slot: Slot::LeftHand, max_hp_bonus: 0, defense_bonus: 0, power_bonus: 2
    });
    game.entities[PLAYER].inventory.push(dagger);

    initialize_fov(tcod, &game.map);

    game.messages.add(
        "Welcome to the Halls of Ruzt - there's no time to change your mind...", RED
    );

    game
}