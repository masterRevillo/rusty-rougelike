use tcod::console::*;
use tcod::map::{FovAlgorithm, Map as FovMap};
use tcod::input::{self, Event, Key, Mouse};

pub struct Tcod {
    pub root: Root,
    pub con: Offscreen,
    pub panel: Offscreen,
    pub fov: FovMap,
    pub key: Key,
    pub mouse: Mouse
}