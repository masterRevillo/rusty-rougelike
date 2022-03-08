use tcod::colors::*;
use tcod::console::*;
use tcod::input::{self, Event, Key, Mouse};
use tcod::map::{FovAlgorithm, Map as FovMap}; //imports the FOV Map object, but renames
                                                // so it doesnt clash with our Map
use std::cmp;
use rand::Rng;

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 75;

const LIMIT_FPS: i32 = 20;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 68;

const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;

const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

const PLAYER: usize = 0;

//colors
const COLOR_DARK_WALL: Color = Color { r: 20, g: 50, b: 10 };
const COLOR_LIGHT_WALL: Color = Color {r: 50,g: 100,b: 20,};
const COLOR_DARK_GROUND: Color = Color {r: 20,g: 10,b: 10,};
const COLOR_LIGHT_GROUND: Color = Color {r: 50,g: 50,b: 10,};

//fov settings
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;

//parameters for dungeon generator
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 32;
const MAX_ROOM_MONSTERS: i32 = 3;

struct Tcod {
    root: Root,
    con: Offscreen,
    panel: Offscreen,
    fov: FovMap,
    key: Key,
    mouse: Mouse
}

type Map = Vec<Vec<Tile>>;

struct Game {
    map: Map,
    messages: Messages,
}

/// This is a generic object: the player, a monster, an item, the stairs...
/// It's always represented by a character on screen.
#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
    name: String,
    blocks: bool,
    alive: bool,
    fighter: Option<Fighter>,
    ai: Option<Ai>
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, color: Color, name: &str, blocks: bool) -> Self {
        Object {
            x: x, 
            y: y, 
            char: char, 
            color: color, 
            name: name.into(), 
            blocks: blocks, 
            alive: false,
            fighter: None,
            ai: None,
        }
    }

    // draw self onto given console
    pub fn draw(&self, con: &mut dyn Console) {             // dyn: Console is a "trait", not a struct. dyn is basically used to announce that its a trait
        con.set_default_foreground(self.color);                  // pointers to traits are double the size of pointers to structs, so there some implications with using it
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None)
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }

    pub fn take_damage(&mut self, damage: i32, game: &mut Game) {
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }
        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, game);
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object, game: &mut Game) {
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            game.messages.add(format!("{} attacks {} for {} hit points", self.name, target.name, damage), WHITE);
            target.take_damage(damage, game);
        } else {
            game.messages.add(format!("{} attacks {}, but it has no effect", self.name, target.name), WHITE);
        }
    }
}

fn player_death(player: &mut Object, game: &mut Game) {
    game.messages.add("You died!", RED);
    player.char = '%';
    player.color = DARK_RED;
}

fn monster_death(monster: &mut Object, game: &mut Game) {
    game.messages.add(format!("{} died", monster.name), ORANGE);
    monster.char = '%';
    monster.color = DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}



#[derive(Clone, Copy, Debug, PartialEq)]
struct Fighter {
    max_hp: i32,
    hp: i32,
    defense: i32,
    power: i32,
    on_death: DeathCallback
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DeathCallback {
    Player,
    Monster,
}

impl DeathCallback {
    fn callback(self, object: &mut Object, game: &mut Game) {
        use DeathCallback::*;
        let callback = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(object, game);
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Ai {
    Basic,
}

fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x,y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy)
    }
}

fn player_move_or_attack(dx: i32, dy: i32, game: &mut Game, objects: &mut [Object]) {
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;

    let target_id = objects.iter().position(|object| object.fighter.is_some() && object.pos() == (x,y));    // position() is an iterator function. It returns the position of the first to match the criteria
    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER, target_id, objects);
            player.attack(target, game);
        }
        None => {
            move_by(PLAYER, dx, dy, &game.map, objects)
        }
    }
}

fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();         // pythagorean path, causes mobs to get stuck on walls

    //normalize to length of 1, then round and convert to integer
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, objects);
}

fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>) {
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);
    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {
            let roll = rand::random::<f32>();
            let mut monster = if roll > 0.9 {
                let mut skeleton = Object::new(x, y, 's', LIGHTEST_SEPIA, "Skeleton", true);
                skeleton.fighter = Some(Fighter {max_hp: 40, hp: 40, defense: 1, power: 6, on_death: DeathCallback::Monster });
                skeleton.ai = Some(Ai::Basic);
                skeleton
            } else if roll > 0.8 { 
                let mut troll = Object::new(x, y, 'T', DARKER_AMBER, "Troll", true);
                troll.fighter = Some(Fighter {max_hp: 35, hp: 35, defense: 4, power: 4, on_death: DeathCallback::Monster });
                troll.ai = Some(Ai::Basic);
                troll
            } else { 
                let mut orc = Object::new(x, y, 'o', DESATURATED_GREEN, "Orc", true);
                orc.fighter = Some(Fighter {max_hp: 10, hp: 10, defense: 0, power: 3, on_death: DeathCallback::Monster });
                orc.ai = Some(Ai::Basic);
                orc
            };
            monster.alive = true;
            objects.push(monster);
        }
    }
}

fn ai_take_turn(monster_id: usize, tcod: &Tcod, game: &mut Game, objects: &mut [Object]) {
    // a basic ai takes a turn. If you can see it, it can see you
    let (monster_x, monster_y) = objects[monster_id].pos();
    if tcod.fov.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            // move towards player if far away
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, &game.map, objects);
        } else {
            // close enough to start a war
            let (monster, player) = mut_two(monster_id, PLAYER, objects);
            monster.attack(player, game);
        }
    }
}

// Takes 2 indexes of an array and returns the mutable item correspoding to both
// This is done by splitting the array into 2 mutable chunks, which contain the
// two desired items. The items are then indexed from the 2 slices, and then returned
fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index != second_index);
    if first_index < second_index {
        let (first_slice, second_slice) = items.split_at_mut(second_index);
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        let (first_slice, second_slice) = items.split_at_mut(first_index);
        (&mut second_slice[0], & mut first_slice[second_index])
    }
}

#[derive(Clone, Copy, Debug)]   // This allows the struct to implement some default behaviors provided by Rust. They are called "traits", but evidently they can be thought of like interfaces
struct Tile {                   // Debug lets us print out the value of the struct; Clone and Copy overrides default assignment strategy of "moving"
    blocked: bool,             
    block_sight: bool,
    explored: bool,
    lit_color: Color,
    dark_color: Color,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
            explored: false,
            lit_color: COLOR_LIGHT_GROUND,
            dark_color: COLOR_DARK_GROUND 
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
            explored: false,
            lit_color: COLOR_LIGHT_WALL,
            dark_color: COLOR_DARK_WALL 
        }
    }
}

fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    if map[x as usize][y as usize].blocked {
        return true;
    }
    objects.
        iter()
        .any(|object| object.blocks && object.pos() == (x,y))
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h
        }
    }

    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    pub fn intersects_with(&self, other: &Rect) -> bool {
        (self.x1 <= other.x2)
            && (self.x2 >= other.x1)
            && (self.y1 <= other.y2)
            && (self.y2 >= other.y1)
    }
}

fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {       // range is inclusive at beginning, but exclusive at end
        for y in (room.y1 +1)..room.y2 {    // so room.x2 does NOT become an empty tile
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn make_map(objects :&mut Vec<Object>) -> Map {
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];    // vec! is a shorthand macro that initializes the Vec and fills it with the specified value
                                                                                        // the syntax is vec![value_to_fill, number_of_entries]
    
    let mut rooms: std::vec::Vec<Rect> = vec![];

    for _ in 0..MAX_ROOMS {
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);

        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);

        // let failed = rooms.iter().any(|other_room| new_room.intersects_with(other_room)); // content of .any() is an anonymous function
        let failed: bool = false;

        if !failed {
            create_room(new_room, &mut map);
            place_objects(new_room, &map, objects);

            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() {
                objects[PLAYER].set_pos(new_x, new_y);
            } else {
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

                if rand::random() {
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }
            rooms.push(new_room);
        }
    }
    map
}

struct Messages {
    messages: Vec<(String, Color)>
}

impl Messages {
    pub fn new() -> Self {
        Self {messages: vec![]}
    }

    pub fn add<T: Into<String>>(&mut self, message: T, color: Color) {
        self.messages.push((message.into(), color))
    }

    // returns an `impl Trait`. basically, it allows you to specify a return type without explicitly describing the type
    // The actual return type just needs to implement the trait specified.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &(String, Color)> {
        self.messages.iter()
    }    
}

fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[Object], fov_recompute: bool) {
    if fov_recompute {
        let player = &objects[PLAYER];
        tcod.fov.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO)
    }
    let mut to_draw: Vec<_> = objects
        .iter()
        .filter(|o| tcod.fov.is_in_fov(o.x, o.y))
        .collect();
    to_draw.sort_by(|o1, o2|{o1.blocks.cmp(&o2.blocks)});
    for object in to_draw {
        object.draw(&mut tcod.con);
    }
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = tcod.fov.is_in_fov(x, y);
            let color = match visible {
                false => game.map[x as usize][y as usize].dark_color,
                true => game.map[x as usize][y as usize].lit_color,
            };
            let explored = &mut game.map[x as usize][y as usize].explored;
            if visible {
                *explored = true;
            }
            if *explored {
                tcod.con.set_char_background(x, y, color, BackgroundFlag::Set);
            }
        }
    }
    // reset GUI panel
    tcod.root.set_default_foreground(WHITE);
    tcod.panel.set_default_background(BLACK);
    tcod.panel.clear();
    // display player stats
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
    render_bar(&mut tcod.panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, LIGHT_GREEN, DARKER_RED);
    // get names at mouse location
    tcod.panel.set_default_foreground(LIGHT_GREY);
    tcod.panel.print_ex(1, 0, BackgroundFlag::None, TextAlignment::Left, get_names_under_mouse(tcod.mouse, objects, &tcod.fov));
    // display message log
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in game.messages.iter().rev() {     // iterate through the messages in reverse order
        let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        tcod.panel.set_default_foreground(color);
        tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }
    blit(
        &tcod.panel, 
        (0,0), 
        (SCREEN_WIDTH, PANEL_HEIGHT), 
        &mut tcod.root, 
        (0, PANEL_Y), 
        1.0, 1.0
    );
    // blit the map
    blit(
        &tcod.con,
        (0, 0),
        (MAP_WIDTH, MAP_HEIGHT),
        &mut tcod.root,
        (0, 0),
        1.0,
        1.0,
    );
}

fn render_bar(
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

fn handle_keys(tcod: &mut Tcod, objects: &mut [Object], game: &mut Game) -> PlayerAction {
    use tcod::input::KeyCode::*;
    use PlayerAction::*;

    let player_alive = objects[PLAYER].alive;
    match (tcod.key, tcod.key.text(), player_alive) {
        (Key {code: Enter, alt: true, ..}, _, _,) => {               // the 2 dots signify that we dont care about the other values of Key. Without them, the code wouldnt compile until all values were supplied
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        }
        (Key { code: Escape, ..}, _, _, )=> return Exit,

        // movement keys
        (Key { code: Up, .. }, _, true ) => {
            player_move_or_attack(0, -1, game, objects);
            TookTurn
        },
        (Key { code: Down, .. }, _, true ) => {
            player_move_or_attack(0, 1, game, objects);
            TookTurn
        },
        (Key { code: Left, .. }, _, true ) => {
            player_move_or_attack(-1, 0, game, objects);
            TookTurn
        },
        (Key { code: Right, .. }, _, true ) => {
            player_move_or_attack(1, 0, game, objects);
            TookTurn
        },
        _ => DidntTakeTurn // everything else
    }

}

fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);
    let names = objects
        .iter()
        .filter(|obj| obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y))
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ")
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

fn main() {
    tcod::system::set_fps(LIMIT_FPS);

    let root = Root::initializer()
        .font("consolas12x12_gs_tc.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("A Rusty Rougelike")
        .init();

    let mut tcod = Tcod {
        root, 
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
        key: Default::default(),                    // default is a trait that can be implemented that gives an object default values
        mouse: Default::default()
    };

    let mut player = Object::new(0, 0, '@', WHITE, "player", true);
    player.alive = true;
    player.fighter = Some(Fighter {
        max_hp: 30,
        hp: 30,
        defense: 2,
        power: 5,
        on_death: DeathCallback::Player
    });
    
    let mut objects = vec![player];

    let mut game = Game {
        map: make_map(&mut objects),
        messages: Messages::new(),
    };

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(
                x,
                y,
                !game.map[x as usize][y as usize].block_sight,
                !game.map[x as usize][y as usize].blocked,
            );
        }
    }

    let mut previous_player_position = (-1, -1);

    game.messages.add(
        "Welcome to the Halls of Ruzt - there's no time to change your mind...", RED
    );

    while !tcod.root.window_closed() {
        // clear offscreen console before drawing anything
        tcod.con.clear();

        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => tcod.key = k,
            _ => tcod.key = Default::default(),
        }
        
        let fov_recompute = previous_player_position != (objects[PLAYER].pos());
        
        render_all(&mut tcod, &mut game, &objects, fov_recompute);
        
        tcod.root.flush();
        
        let player = &mut objects[PLAYER];
        previous_player_position = player.pos();
        let player_action = handle_keys(&mut tcod, &mut objects, &mut game);
        if player_action == PlayerAction::Exit {
            break;
        }
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, &tcod, &mut game, &mut objects)
                }
            }
            // old logic, kept for pointer comparison syntax:
            // for object in &objects {
            //     // only if object is not player
            //     if (object as *const _) != (&objects[PLAYER] as *const _) { // *const _ does a pointer comparison
            //         ai_take_turn(id, &tcod, &game, &mut objects)
            //     }
            // }
        }
    }
}