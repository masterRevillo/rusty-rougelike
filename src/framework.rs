use tcod::console::*;
use tcod::map::{Map as FovMap};
use tcod::input::{Key, Mouse};

pub struct Tcod {
    pub root: Root,
    pub con: Offscreen,
    pub panel: Offscreen,
    pub fov: FovMap,
    pub key: Key,
    pub mouse: Mouse
}