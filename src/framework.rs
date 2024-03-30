use bracket_lib::color::{BLACK, RGBA, WHITE};
use bracket_lib::pathfinding::field_of_view;
use bracket_lib::prelude::{BTerm, Point};
use crate::map::mapgen::{Map, MAP_HEIGHT, MAP_WIDTH};
use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

pub struct GameFramework {
    pub con: BTerm,
    // pub root: Root,
    // pub con: Offscreen,
    // pub panel: Offscreen,
    pub fov: FovMap,
    // pub key: Key,
    // pub mouse: Mouse
}

impl GameFramework {

    pub fn display_menu<T: AsRef<str>>(&mut self, header: &str, options: &[T], width: i32) {
        assert!(options.len() <= 26, "Cannot have more than 26 options in the menu");
        // calculate total height for the header after wrapping, plus a line for each menu option
        // let header_height = if header.is_empty() {
        //     0
        // } else {
        //     con.get_height_rect(0, 0, width, SCREEN_HEIGHT, header)
        // };
        let height = options.len() as i32 + 1;

        // create offscreen console for the menu window
        // let mut window = Offscreen::new(width, height);
        // window.set_default_foreground(WHITE);
        // window.print_rect_ex(0, 0, width, height, BackgroundFlag::None, TextAlignment::Left, header);
        self.con.draw_box(10, 10, width, height, RGBA::from(WHITE), RGBA::from(BLACK));

        // print the options
        for (index, option_text) in options.iter().enumerate() {
            let menu_letter = (b'a' + index as u8) as char;
            let text = format!("({}) {}", menu_letter, option_text.as_ref());
            self.con.print_color(0, 1 + index as i32, RGBA::from(WHITE), RGBA::from(BLACK), text.as_str());
        }

        let x = SCREEN_WIDTH / 2 - width / 2;
        let y = SCREEN_HEIGHT / 2 - height / 2;
    }
}

pub struct FovMap {
    pub visible_tiles: Vec<Point>,
    pub range: i32
}

impl FovMap {
    pub fn new() -> Self {
        FovMap {
            visible_tiles: vec![],
            range: 8
        }
    }

    pub fn is_in_fov(&self, x: i32, y: i32) -> bool {
        self.visible_tiles.contains(&Point::new(x, y))
    }

    pub fn compute_fov(&mut self, x: i32, y: i32, radius: i32, map: &Map) {
        self.visible_tiles.clear();
        self.visible_tiles = field_of_view(Point::new(x, y), radius, &*map);
        self.visible_tiles.retain(|p| p.x >= 0 && p.x < MAP_WIDTH && p.y >= 0 && p.y < MAP_HEIGHT );
    }

    pub fn set(&mut self, x: i32, y: i32, transparent: bool, walkable: bool) {
        if transparent {
            self.visible_tiles.push(Point::new(x, y))
        }
    }
}

