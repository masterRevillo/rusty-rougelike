use crate::items::item::Item;
use std::borrow::BorrowMut;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::ptr::null;
use bracket_lib::prelude::BTerm;

// use tcod::{BackgroundFlag, Console, TextAlignment};
// use tcod::colors::{DARK_RED, RED, SKY, WHITE};

use crate::{AudioEventProcessor, Camera, Entity, EventBus, EventLogProcessor, GameEngine, GameOccurrenceEventProcessor,  load_configs, make_map, MAP_HEIGHT, MAP_WIDTH, menu, Messages, msgbox, SCREEN_HEIGHT, SCREEN_WIDTH, GameFramework};
use crate::entities::entity::EntityType;
use crate::entities::entity::EntityType::{Player};
use crate::entities::equipment::Equipment;
use crate::entities::fighter::Fighter;
use crate::entities::slot::Slot;
use crate::game_engine::{GameState, PLAYER};
use crate::map::mapgen::Map;
use crate::util::color::{BLUE, Color, RED, WHITE};
use crate::util::death_callback::DeathCallback;

pub fn main_menu(framework: &mut GameFramework) {
    // let img = tcod::image::Image::from_file("desert.png").ok().expect("Background image not found");

    while !framework.con.quitting {
        // tcod::image::blit_2x(&img, (1800,800), (-1,-1), &mut tcod.root, (0,0));
        //
        // tcod.root.set_default_foreground(DARK_RED);
        // tcod.root.print_ex(SCREEN_WIDTH / 2, SCREEN_HEIGHT / 2 - 6, BackgroundFlag::None, TextAlignment::Center, "THE HALLS OF RUZT");
        // tcod.root.print_ex(SCREEN_WIDTH / 2, SCREEN_HEIGHT / 2 - 4, BackgroundFlag::None, TextAlignment::Center, "By Rev");

        let choices = &["Play a new game", "Continue last game", "Quit"];
        let choice = menu("", choices, 24, &mut framework.con);

        match choice {
            Some(0) => {
                let mut game= new_game();
                game.run_game_loop(framework);
            }
            Some(1) => {
                match load_game() {
                    Ok(mut game) => {
                        // initialize_fov(framework, &game.map);
                        game.run_game_loop(framework);
                    },
                    Err(_e) => {
                        msgbox("\n No saved game to load.\n", 24, &mut framework.con);
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

pub fn new_game() -> GameEngine {
    let config = load_configs();
    let mut player = Entity::new(0, 0, '@', Color::from(WHITE), "player", true, Player);
    player.alive = true;
    player.fighter = Some(Fighter {
        base_max_hp: 30,
        hp: 30,
        base_defense: 2,
        base_power: 3,
        xp: 200,
        on_death: DeathCallback::Player
    });

    let entities = vec![player];

    let mut game = GameEngine {
        map: Map::new(),
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
        },
        game_state: Box::new(GameState::main())
    };
    let map = make_map(game.borrow_mut(), 1);
    game.map = map;


    // game.set_audio_engine(config);
    // game.play_background_music();

    let mut dagger = Entity::new(
        0,
        0,
        '-',
        Color::from(BLUE),
        "dagger",
        false,
        EntityType::Item
    );
    dagger.item = Some(Item::Sword);
    dagger.equipment = Some(Equipment {
        equipped: true, slot: Slot::LeftHand, max_hp_bonus: 0, defense_bonus: 0, power_bonus: 2
    });
    game.entities[PLAYER].inventory.push(dagger);

    // initialize_fov(tcod, &game.map);

    game.messages.add(
        "Welcome to the Halls of Ruzt - there's no time to change your mind...", Color::from(RED)
    );

    game
}

// return type is a result, which can either be a success, or a type that implements the error type.
pub fn save_game(game: &mut GameEngine) -> Result<(), Box<dyn Error>> {
    let save_data = serde_json::to_string(&game)?;       // the ? gets the success value, or returns immediately with the error type
    let mut file = File::create("savegame")?;
    file.write_all(save_data.as_bytes())?;
    Ok(())
}
