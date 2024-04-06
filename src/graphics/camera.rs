use crate::Entity;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Camera {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub map_width: i32,
    pub map_height: i32
}

impl Camera {
    pub fn get_pos_in_camera(&self, x: i32, y: i32) -> (i32, i32) {
        (x + self.x, y + self.y)
    }

    pub fn get_map_pos(&self, x: i32, y: i32) -> (i32, i32) {
        (x - self.x, y - self.y)
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        0 <= x && x < self.width && 0 <= y && y < self.height
    }

    pub fn update(&mut self, entity: &Entity) {
        let x = -entity.x + (self.width / 2);
        let y = -entity.y + (self.height / 2);

        self.x = x;
        self.y = y;
    }
}
