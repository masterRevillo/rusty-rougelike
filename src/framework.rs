use bracket_lib::pathfinding::field_of_view;
use bracket_lib::prelude::{BTerm, Point};
use crate::map::mapgen::{Map, MAP_HEIGHT, MAP_WIDTH};

pub struct GameFramework {
    pub con: BTerm,
    // pub root: Root,
    // pub con: Offscreen,
    // pub panel: Offscreen,
    pub fov: FovMap,
    // pub key: Key,
    // pub mouse: Mouse
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

