use bracket_lib::color::{BLACK, DARK_GOLDENROD, RGBA, WHITE};
use bracket_lib::prelude::{BTerm, field_of_view, letter_to_option};
use bracket_lib::terminal::Console;

use crate::{Entity, GameFramework, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::game_engine::{GameEngine, LEVEL_SCREEN_WIDTH, PLAYER};
use crate::map::mapgen::{Map, MAP_HEIGHT, MAP_WIDTH};

pub const INVENTORY_WIDTH: i32 = 50;


pub const BAR_WIDTH: i32 = 20;
pub const PANEL_HEIGHT: i32 = 7;
pub const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;

pub const MSG_X: i32 = BAR_WIDTH + 2;
pub const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
pub const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

pub fn initialize_fov(framework: &mut GameFramework, map: &Map) {
    for y in 0..MAP_HEIGHT as usize {
        for x in 0..MAP_WIDTH as usize {
            framework.fov.set(
                x as i32,
                y as i32,
                !map.tiles[x][y].block_sight,
                !map.tiles[x][y].blocked,
            );
        }
        // field_of_view()
    }
}

pub fn inventory_menu(inventory: &[Entity], header: &str, con: &mut BTerm) -> Option<usize> {
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
    } else {
        inventory.iter().map(|item| {
            match item.equipment {
                Some(equipment) if equipment.equipped => {
                    format!("{} (on {})", item.name, equipment.slot)
                }
                _ => item.name.clone()
            }

        }).collect()
    };

    let inventory_index = menu(header, &options, INVENTORY_WIDTH, con);

    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

pub fn msgbox(text: &str, width: i32, con: &mut BTerm) {
    let options: &[&str] = &[];
    menu(text, options, width, con);
}

pub fn get_names_under_mouse(game_framework: &mut GameFramework, objects: &[Entity]) -> String {
    let (x, y) = game_framework.con.mouse_pos;
    let names = objects
        .iter()
        // .filter(|obj| obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y))
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ")
}


pub fn render_inventory_menu(framework: &mut GameFramework, game: &mut GameEngine, header: &str) {
    let inventory = &game.entities[PLAYER].inventory;
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
    } else {
        inventory.iter().map(|item| {
            match item.equipment {
                Some(equipment) if equipment.equipped => {
                    format!("{} (on {})", item.name, equipment.slot)
                }
                _ => item.name.clone()
            }
        }).collect()
    };
    framework.display_menu(
        header,
        &options,
        INVENTORY_WIDTH,
    );
}


pub fn render_level_up_menu(framework: &mut GameFramework, game: &mut GameEngine, header: &str) {
    let player = &mut game.entities[PLAYER];

    //TODO add message back in:
    // game.messages.add(format!("Your experience has increased. You are now level {}!", player.level), YELLOW);
    let fighter = player.fighter.as_mut().unwrap();

    framework.display_menu(
        header,
        &[
            format!("Constitution (+20 HP, from {})", fighter.base_max_hp),
            format!("Strength (+1 attack, from {})", fighter.base_power),
            format!("Agility (+1 defense, from {})", fighter.base_defense),
        ],
        LEVEL_SCREEN_WIDTH,
    );
}

pub fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, console: &mut BTerm) -> Option<usize> {
    assert!(options.len() <= 26, "Cannot have more than 26 options in the menu");
    // calculate total height for the header after wrapping, plus a line for each menu option
    let height = options.len() as i32 + 1;

    // create offscreen console for the menu window
    console.draw_box(0, 0, width, height, RGBA::from(WHITE), RGBA::from(BLACK));

    // print the options
    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        console.print_color(0, 1 + index as i32, RGBA::from(WHITE), RGBA::from(BLACK), text.as_str());
    }

    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    // blit(&window, (0,0), (width, height), console, (x, y), 1.0, 0.7);

    // present the root console and wait for key
    // console.flush();
    // let key = console.wait_for_keypress(true);
    let mut key = None;

    let mut choice = None;

    while key.is_none() {
        key = console.key;
        match key {
            None => {}
            Some(v) => {
                let sel = letter_to_option(v);
                if sel > -1 && sel < options.len() as i32 {
                    choice = Some(sel as usize)
                } else {
                    choice = None
                }
            }
        }
    }
    return choice
}



pub fn render_bar(
    con: &mut BTerm,
    x: i32,
    y: i32,
    total_width: i32,
    name: &str,
    value: i32,
    maximum: i32,
    bar_color: RGBA,
    back_color: RGBA,
) {
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;

    // render the background
    con.draw_box(x, y, total_width, 1, back_color, RGBA::from(BLACK));

    // render the bar on top
    // con.set_default_background(bar_color);
    if bar_width > 0 {
        con.draw_box(x, y, bar_width, 1, bar_color, RGBA::from(BLACK));
    }

    // then some text with values
    con.print_color_centered_at(
        x + total_width / 2 ,
        y,
        RGBA::from(DARK_GOLDENROD),
        RGBA::from(BLACK),
        &format!("{}: {}/{}", name, value, maximum)
    )
}
