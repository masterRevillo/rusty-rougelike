use tcod::{BackgroundFlag, Color, Console, TextAlignment};
use tcod::colors::{DARKER_SEPIA, WHITE};
use tcod::console::{blit, Offscreen, Root};
use tcod::input::Mouse;

use crate::{Entity, FovMap, Map, MAP_HEIGHT, MAP_WIDTH, SCREEN_HEIGHT, SCREEN_WIDTH, Tcod};

pub const INVENTORY_WIDTH: i32 = 50;


pub const BAR_WIDTH: i32 = 20;
pub const PANEL_HEIGHT: i32 = 7;
pub const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;

pub const MSG_X: i32 = BAR_WIDTH + 2;
pub const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
pub const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

pub fn initialize_fov(tcod: &mut Tcod, map: &Map) {
    for y in 0..MAP_HEIGHT as usize {
        for x in 0..MAP_WIDTH as usize {
            tcod.fov.set(
                x as i32,
                y as i32,
                !map[x][y].block_sight,
                !map[x][y].blocked,
            );
        }
    }
}

pub fn inventory_menu(inventory: &[Entity], header: &str, root: &mut Root) -> Option<usize> {
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

    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);

    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

pub fn msgbox(text: &str, width: i32, root: &mut Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}

pub fn get_names_under_mouse(mouse: Mouse, objects: &[Entity], fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);
    let names = objects
        .iter()
        .filter(|obj| obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y))
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ")
}


pub fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
    assert!(options.len() <= 26, "Cannot have more than 26 options in the menu");
    // calculate total height for the header after wrapping, plus a line for each menu option
    let header_height = if header.is_empty() {
        0
    } else {
        root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header)
    };
    let height = options.len() as i32 + header_height;

    // create offscreen console for the menu window
    let mut window = Offscreen::new(width, height);
    window.set_default_foreground(WHITE);
    window.print_rect_ex(0, 0, width, height, BackgroundFlag::None, TextAlignment::Left, header);

    // print the options
    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        window.print_ex(0, header_height + index as i32, BackgroundFlag::None, TextAlignment::Left, text);
    }

    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    blit(&window, (0,0), (width, height), root, (x, y), 1.0, 0.7);

    // present the root console and wait for key
    root.flush();
    let key = root.wait_for_keypress(true);

    // convert the ascii code to an index; return if it matches an option
    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn render_bar(
    panel: &mut Offscreen,
    x: i32,
    y: i32,
    total_width: i32,
    name: &str,
    value: i32,
    maximum: i32,
    bar_color: Color,
    back_color: Color,
) {
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;

    // render the background
    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

    // render the bar on top
    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }

    // then some text with values
    panel.set_default_foreground(DARKER_SEPIA);
    panel.print_ex(
        x + total_width / 2 , y, BackgroundFlag::None, TextAlignment::Center, &format!("{}: {}/{}", name, value, maximum)
    )
}