use tcod::colors::*;
use tcod::console::*;
use tcod::input::{self, Event, Key, Mouse};
use tcod::map::{FovAlgorithm, Map as FovMap}; //imports the FOV Map object, but renames
                                                // so it doesnt clash with our Map
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use serde::{Deserialize, Serialize};
use std::cmp;
use rand::Rng;
use rand::distributions::{IndependentSample, Weighted, WeightedChoice};

mod namegen;

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 75;

const INVENTORY_WIDTH: i32 = 50;

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
// const COLOR_DARK_WALL: Color = Color { r: 20, g: 50, b: 10 };
// const COLOR_LIGHT_WALL: Color = Color {r: 50,g: 100,b: 20,};
// const COLOR_DARK_GROUND: Color = Color {r: 20,g: 10,b: 10,};
// const COLOR_LIGHT_GROUND: Color = Color {r: 170, g: 131, b: 96};
const COLOR_DARK_WALL: Color = DARKEST_RED;
const COLOR_LIGHT_WALL: Color = Color {r: 127, g: 30, b: 20};
const COLOR_DARK_GROUND: Color = Color {r: 20,g: 10,b: 10,};
const COLOR_LIGHT_GROUND: Color = Color {r: 170, g: 131, b: 96};

//fov settings
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;

//parameters for dungeon generator
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 32;

//parameters for items
const HEAL_AMOUNT: i32 = 4;
const LIGHTNING_DAMAGE: i32 = 40;
const LIGHTNING_RANGE: i32 = 5;
const CONFUSE_RANGE: i32 = 8;
const CONFUSE_NUM_TURNS: i32 = 10;
const FIREBALL_RADIUS: i32 = 3;
const FIREBALL_DAMAGE: i32 = 12;

//parameters for leveling up
const LEVEL_UP_BASE: i32 = 200;
const LEVEL_UP_FACTOR: i32 = 150;
const LEVEL_SCREEN_WIDTH: i32 = 40;
const STATS_SCREEN_WIDTH: i32 = 30;

struct Tcod {
    root: Root,
    con: Offscreen,
    panel: Offscreen,
    fov: FovMap,
    key: Key,
    mouse: Mouse
}

type Map = Vec<Vec<Tile>>;

#[derive(Serialize, Deserialize)]
struct Game {
    map: Map,
    messages: Messages,
    //TODO: move inventory out of the game struct
    inventory: Vec<Object>,
    dungeon_level: u32,
}

struct Transition {
    level: u32,
    value: u32,
}

const MAX_MONSTERS_TRANSITION: &[Transition] = &[
    Transition { level: 1, value: 2 },
    Transition { level: 4, value: 3 },
    Transition { level: 6, value: 5 },
];

const TROLL_CHANCE_TRANSITION: &[Transition] = &[
    Transition{ level: 3, value: 15 },
    Transition{ level: 5, value: 30 },
    Transition{ level: 7, value: 60 },
];

const SKELETON_CHANCE_TRANSITION: &[Transition] = &[
    // Transition{ level: 1, value: 0 },
    Transition{ level: 3, value: 5 },
    Transition{ level: 5, value: 10 },
    Transition{ level: 7, value: 30 },
];

const MAX_ITEMS_TRANSITION: &[Transition] = &[
    Transition{ level: 1, value: 1 },
    Transition{ level: 4, value: 2 },
];

const ROOM_OVERLAP_TRANSITION: &[Transition] = &[
    Transition{ level: 3, value: 1 },
];

/// This is a generic object: the player, a monster, an item, the stairs...
/// It's always represented by a character on screen.
#[derive(Debug, Serialize, Deserialize)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
    name: String,
    blocks: bool,
    alive: bool,
    fighter: Option<Fighter>,
    ai: Option<Ai>,
    item: Option<Item>,
    always_visible: bool,
    level: i32,
    equipment: Option<Equipment>,
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
            item: None,
            always_visible: false,
            level: 1,
            equipment: None,
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

    pub fn take_damage(&mut self, damage: i32, game: &mut Game) -> Option<i32>{
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }
        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, game);
                return Some(fighter.xp);
            }
        }
        None
    }

    pub fn attack(&mut self, target: &mut Object, game: &mut Game) {
        let damage = self.power(game) - target.defense(game);
        if damage > 0 {
            game.messages.add(format!("{} attacks {} for {} hit points", self.name, target.name, damage), WHITE);
            if let Some(xp) = target.take_damage(damage, game) {
                self.fighter.as_mut().unwrap().xp += xp;
            }
        } else {
            game.messages.add(format!("{} attacks {}, but it has no effect", self.name, target.name), WHITE);
        }
    }

    pub fn heal(&mut self, amount: i32, game: &Game) {
        let max_hp = self.max_hp(game);
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > max_hp {
                fighter.hp = max_hp;
            }
        }
    }

    pub fn distance(&self, x: i32, y:i32) -> f32{
        (((x - self.x).pow(2) + (y - self.y).pow(2)) as f32).sqrt()
    }

    pub fn equip(&mut self, messages: &mut Messages) {
        if self.item.is_none() {
            messages.add(format!("Cannot equip {:?} because it's not an Item.", self ), RED);
            return;
        };
        if let Some(ref mut equipment) = self.equipment {
            if !equipment.equipped {
                equipment.equipped = true;
                messages.add(format!("Equipped {} on {}.", self.name, equipment.slot), LIGHT_GREEN);
            }
        } else {
            messages.add(format!("Cannot equip {:?} because it's not an Equipment.", self ), RED);
        }
    }

    pub fn unequip(&mut self, messages: &mut Messages) {
        if self.item.is_none() {
            messages.add(format!("Cannot unequip {:?} because it's not an Item.", self ), RED);
            return;
        }
        if let Some(ref mut equipment) = self.equipment {
            if equipment.equipped {
                messages.add(format!("Unequipped {} from {}.", self.name, equipment.slot), LIGHT_YELLOW);
            }
        } else {
            messages.add(format!("Cannot unequip {:?} because it's not an Equipment.", self ), RED); 
        }
    }

    pub fn power(&self, game: &Game) -> i32 {
        let base_power = self.fighter.map_or(0, |f| f.base_power);
        let bonus: i32 = self.get_all_equipped(game).iter().map(|e| e.power_bonus).sum();
        base_power + bonus
    }

    pub fn defense(&self, game: &Game) -> i32 {
        let base_defense = self.fighter.map_or(0, |f| f.base_defense);
        let bonus: i32 = self.get_all_equipped(game).iter().map(|e| e.defense_bonus).sum();
        base_defense + bonus
    }

    pub fn max_hp(&self, game: &Game) -> i32 {
        let base_max_hp = self.fighter.map_or(0, |f| f.base_max_hp);
        let bonus: i32 = self.get_all_equipped(game).iter().map(|e| e.max_hp_bonus).sum();
        base_max_hp + bonus
    }

    pub fn get_all_equipped(&self, game: &Game) -> Vec<Equipment>{
        // TODO: remove this, as every entity should have their own inventory
        if self.name == "player" {
            game.inventory.iter()
                .filter(|e| e.equipment.map_or(false, |e| e.equipped))
                .map(|e| e.equipment.unwrap())
                .collect()
        } else {
            vec![]
        }
    }
}

fn level_up(tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) {
    let player = &mut objects[PLAYER];
    let level_up_xp = LEVEL_UP_BASE + LEVEL_UP_FACTOR * player.level;
    if player.fighter.as_ref().map_or(0, |f| f.xp) >= level_up_xp {
        player.level += 1;
        game.messages.add(format!("Your experience has increased. You are now level {}!", player.level), YELLOW);
        let fighter = player.fighter.as_mut().unwrap();
        let mut choice = None;
        while choice.is_none() {
            choice = menu(
                "Level up! Choose a stat to increase: \n",
                &[
                    format!("Constitution (+20 HP, from {}", fighter.base_max_hp),
                    format!("Strength (+1 attack, from {}", fighter.base_power),
                    format!("Agility (+1 base_defense, from {}", fighter.base_defense),
                ],
                LEVEL_SCREEN_WIDTH,
                &mut tcod.root
            )
        }
        fighter.xp -= level_up_xp;
        match choice.unwrap() {
            0 => {
                fighter.base_max_hp += 20;
                fighter.hp += 20;
            }
            1 => {
                fighter.base_power += 1;
            }
            2 => {
                fighter.base_defense += 1;
            }
            _ => unreachable!()
        }
    } 
}

fn player_death(player: &mut Object, game: &mut Game) {
    game.messages.add("You died!", RED);
    player.char = '%';
    player.color = DARK_RED;
}

fn monster_death(monster: &mut Object, game: &mut Game) {
    game.messages.add(format!("{} died. It gives you {} xp.", monster.name, monster.fighter.unwrap().xp), ORANGE);
    monster.char = '%';
    monster.color = DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}


//TODO spilt xp store for player and drop xp into different values
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Fighter {
    base_max_hp: i32,
    hp: i32,
    base_defense: i32,
    base_power: i32,
    xp: i32,
    on_death: DeathCallback
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Ai {
    Basic,
    Confused {                  // enum values can hold data. Dope
        previous_ai: Box<Ai>,
        num_turns: i32
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Item {
    Heal,
    Lightning,
    Confuse,
    Fireball,
    Artifact {name: String, value: i32},
    Sword,
    Shield,
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

fn pick_item_up(object_id: usize, game: &mut Game, objects: &mut Vec<Object> ) {
    if game.inventory.len() >= 26 {
        game.messages.add(format!("Your pickets are full - you can't pickup the {}", objects[object_id].name), RED)
    } 
    else {
        let item = objects.swap_remove(object_id);
        game.messages.add(format!("You picked up the {}", item.name), GREEN);
        let index = game.inventory.len();
        let slot = item.equipment.map(|e| e.slot);
        game.inventory.push(item);

        // equip picked up item if it is equipment and the slot is open
        if let Some(slot) = slot {
            if get_equipped_id_in_slot(slot, &game.inventory).is_none() {
                game.inventory[index].equip(&mut game.messages);
            }
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Equipment {
    slot: Slot,
    equipped: bool,
    max_hp_bonus: i32,
    power_bonus: i32,
    defense_bonus: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum Slot {
    LeftHand,
    RightHand,
    Head,
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Slot::LeftHand => write!(f, "left hand"),
            Slot::RightHand => write!(f, "right hand"),
            Slot::Head => write!(f, "head"),
        }
    }
}

fn from_dungeon_level(table: &[Transition], level: u32) -> u32 {
    table
        .iter()
        .rev()
        .find(|transition| level >= transition.level)
        .map_or(0, |transition| transition.value)
}

fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>, level: u32) {
    let max_monsters = from_dungeon_level(MAX_MONSTERS_TRANSITION, level);

    let num_monsters = rand::thread_rng().gen_range(0, max_monsters + 1);

    let troll_chance = from_dungeon_level(TROLL_CHANCE_TRANSITION, level);
    let skeleton_chance = from_dungeon_level(SKELETON_CHANCE_TRANSITION, level);
    let monster_chances = &mut [
        Weighted {
            weight: 80,
            item: "orc",
        },
        Weighted {
            weight: troll_chance,
            item: "troll"
        },
        Weighted {
            weight: skeleton_chance,
            item: "skeleton"
        }
    ];
    let monster_choice = WeightedChoice::new(monster_chances);

    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {
            let mut monster = match monster_choice.ind_sample(&mut rand::thread_rng()) {
                "skeleton" => {
                    let mut skeleton = Object::new(x, y, 's', LIGHTEST_SEPIA, "Skeleton", true);
                    skeleton.fighter = Some(Fighter {base_max_hp: 25, hp: 25, base_defense: 1, base_power: 6, xp: 200, on_death: DeathCallback::Monster });
                    skeleton.ai = Some(Ai::Basic);
                    skeleton
                },
                "troll" => {
                    let mut troll = Object::new(x, y, 'T', DARKER_AMBER, "Troll", true);
                    troll.fighter = Some(Fighter {base_max_hp: 30, hp: 30, base_defense: 2, base_power: 4, xp: 100, on_death: DeathCallback::Monster });
                    troll.ai = Some(Ai::Basic);
                    troll
                },
                "orc" => {
                    let mut orc = Object::new(x, y, 'o', DESATURATED_GREEN, "Orc", true);
                    orc.fighter = Some(Fighter {base_max_hp: 10, hp: 10, base_defense: 0, base_power: 3, xp: 35, on_death: DeathCallback::Monster });
                    orc.ai = Some(Ai::Basic);
                    orc
                },
                _ => unreachable!()   
            };
            monster.alive = true;
            objects.push(monster);
        }
    }

    let item_chances = &mut [
        Weighted {
            weight: 35,
            item: Item::Heal
        },
        Weighted {
            weight: from_dungeon_level(&[Transition{ level: 4, value:25 }], level),
            item: Item::Lightning
        },
        Weighted {
            weight: from_dungeon_level(&[Transition{ level: 2, value:10 }], level),
            item: Item::Confuse
        },
        Weighted {
            weight: from_dungeon_level(&[Transition{ level: 6, value:25 }], level),
            item: Item::Fireball
        },
        Weighted {
            weight: from_dungeon_level(&[
                Transition{ level: 2, value:0 }, Transition{ level: 2, value:5 }, Transition{ level: 5, value: 15 } 
                ], level
            ),
            item: Item::Artifact{name:"".to_string(), value:0}
        },
        Weighted {
            weight: from_dungeon_level(&[Transition { level: 4, value: 5 }], level), 
            item: Item::Sword
        },
        Weighted {
            weight: from_dungeon_level(&[Transition { level: 8, value: 15 }], level), 
            item: Item::Shield
        }
    ];
    let item_choice = WeightedChoice::new(item_chances);

    let max_items = from_dungeon_level(MAX_ITEMS_TRANSITION, level);
    let num_items = rand::thread_rng().gen_range(0, max_items + 1);

    for _ in 0..num_items {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {
            let mut item = match item_choice.ind_sample(&mut rand::thread_rng()) {
                Item::Heal => {
                    let mut object = Object::new(x, y, '!', VIOLET, "health potion", false);
                    object.item = Some(Item::Heal);
                    object
                },
                Item::Lightning => {
                    let mut object = Object::new(x, y, '#', LIGHT_YELLOW, "scroll of lightning bolt", false);
                    object.item = Some(Item::Lightning);
                    object
                },
                Item::Fireball => {
                    let mut object = Object::new(x, y, '#', LIGHT_RED, "scroll of firball", false);
                    object.item = Some(Item::Fireball);
                    object
                },
                Item::Confuse => {
                    let mut object = Object::new(x, y, '#', LIGHTEST_YELLOW, "scroll of confusion", false);
                    object.item = Some(Item::Confuse);
                    object
                },
                Item::Artifact{name: _, value: _} => {
                    let mut object = Object::new(x, y, '{', GOLD, "artifact", false);
                    object.item = Some(
                        Item::Artifact{
                            name: namegen::generate_artifact_name(2,7), 
                            value: 250 * rand::thread_rng().gen_range(1, 30)
                        }
                    );
                    object
                },
                Item::Sword => {
                    let mut object = Object::new(x, y, '/', SKY, "sword", false);
                    object.item = Some(Item::Sword);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::RightHand, power_bonus: 3, defense_bonus: 0, max_hp_bonus: 0});
                    object
                },
                Item::Shield => {
                    let mut object = Object::new(x, y, '[', DARK_ORANGE, "shield", false);
                    object.item = Some(Item::Shield);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::LeftHand, power_bonus: 0, defense_bonus: 1, max_hp_bonus: 0});
                    object
                }
                 
            };
            item.always_visible = true;
            objects.push(item);
        }
    }
}

fn ai_take_turn(monster_id: usize, tcod: &Tcod, game: &mut Game, objects: &mut [Object]) {
    use Ai::*;
    if let Some(ai) = objects[monster_id].ai.take() {               // take() removes to the option from Option - it then becomes empty
        let new_ai = match ai {
            Basic => ai_basic(monster_id, tcod, game, objects),
            Confused {
                previous_ai,
                num_turns   
            } => ai_confused(monster_id, tcod, game, objects, previous_ai, num_turns)
        };
        objects[monster_id].ai = Some(new_ai);                      // the AI is then put back here
    }
}

fn ai_basic(monster_id: usize, tcod: &Tcod, game: &mut Game, objects: &mut [Object]) -> Ai {
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
    Ai::Basic
}

fn ai_confused(monster_id:usize, _tcod: & Tcod, game: &mut Game, objects: &mut [Object], previous_ai: Box<Ai>, num_turns: i32) -> Ai {
    let x = rand::thread_rng().gen_range(0, MAP_WIDTH);
    let y = rand::thread_rng().gen_range(0, MAP_HEIGHT);
    move_towards(monster_id, x, y, &game.map, objects);
    if num_turns == 0 {
        game.messages.add(format!("The {} is no longer confused", objects[monster_id].name), RED);
        *previous_ai
    } else {
        Ai::Confused{ previous_ai: previous_ai, num_turns: num_turns - 1}
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]   // This allows the struct to implement some default behaviors provided by Rust. They are called "traits", but evidently they can be thought of like interfaces
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

fn make_map(objects :&mut Vec<Object>, level: u32) -> Map {
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];    // vec! is a shorthand macro that initializes the Vec and fills it with the specified value
                                                                                        // the syntax is vec![value_to_fill, number_of_entries]
    
    let mut rooms: std::vec::Vec<Rect> = vec![];

    // TODO: this will break if the player is not the first object in the list... that should change
    // this compares the pointers to both objects and verifies that they are the same value
    assert_eq!(&objects[PLAYER] as *const _, &objects[0] as *const _);
    objects.truncate(1);

    for _ in 0..MAX_ROOMS {
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);

        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);

        let failed = match from_dungeon_level(ROOM_OVERLAP_TRANSITION, level) {
            0 => rooms.iter().any(|other_room| new_room.intersects_with(other_room)),
            _ => false
        };

        if !failed {
            create_room(new_room, &mut map);
            place_objects(new_room, &map, objects, level);

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
    let (last_room_x, last_room_y) = rooms[rooms.len() -1].center();
    let mut stairs = Object::new(last_room_x, last_room_y, '<', WHITE, "stairs", false);
    stairs.always_visible = true;
    objects.push(stairs);
    map
}

fn next_level(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<Object>) {
    game.messages.add("You rest for a minute and recover your strength", VIOLET);
    let heal_hp = objects[PLAYER].max_hp(game) / 2;
    objects[PLAYER].heal(heal_hp, game);
    game.messages.add("You descend deeper into the dungeon ...", RED);
    game.dungeon_level += 1;
    game.map = make_map(objects, game.dungeon_level);
    initialize_fov(tcod, &game.map);
}

enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
}

fn use_item(inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) {
    use Item::*;
    if let Some(item) = &game.inventory[inventory_id].item {
        let on_use = match item {
            Heal => cast_heal,
            Lightning => cast_lightning,
            Confuse => cast_confuse,
            Fireball => cast_fireball,
            Artifact{name: _, value: _} => examine_artifact,
            Sword => toggle_equipment,
            Shield => toggle_equipment,
        };
        match on_use(inventory_id, tcod, game, objects) {
            UseResult::UsedUp => {
                game.inventory.remove(inventory_id);
            }
            UseResult::UsedAndKept => {}
            UseResult::Cancelled => {
                game.messages.add("Cancelled", WHITE);
            }
        }
    } else {
        game.messages.add(format!("The {} cannot be used.", game.inventory[inventory_id].name), WHITE);
    }
}

fn drop_item(inventory_id: usize, game: &mut Game, objects: &mut Vec<Object>) {
    let mut item = game.inventory.remove(inventory_id);
    if item.equipment.is_some() {
        item.unequip(&mut game.messages);
    }
    item.set_pos(objects[PLAYER].x, objects[PLAYER].y);
    game.messages.add(format!("You dropped the {}.", item.name), YELLOW);
    objects.push(item);
}

fn cast_heal(
    _inventory_id: usize,
    _tcod: &mut Tcod,
    game: &mut Game,
    objects: &mut [Object]
) -> UseResult {
    let player = &mut objects[PLAYER];
    if let Some(fighter) = player.fighter {
        if fighter.hp == player.max_hp(game) {
            game.messages.add("You're already at full health. ", RED);
            return UseResult::Cancelled;
        }
        game.messages.add("Your wounds feel a bit better", LIGHT_VIOLET);
        player.heal(HEAL_AMOUNT, game);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

fn cast_lightning(
    _inventory_id: usize,
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &mut [Object]
) -> UseResult {
    let monster_id = closest_monster(tcod, objects, LIGHTNING_RANGE);
    if let Some(monster_id) = monster_id {
        game.messages.add(
            format!("A lightning bolt strikes the {}! It deals {} points of damage.", objects[monster_id].name, LIGHTNING_DAMAGE), 
            LIGHT_BLUE
        );
        if let Some(xp) = objects[monster_id].take_damage(LIGHTNING_DAMAGE, game) {
            // TODO: determine attacker and award xp to them, not automatically to player
            objects[PLAYER].fighter.as_mut().unwrap().xp += xp;
        }
        UseResult::UsedUp
    } else {
        game.messages.add("No enemies are within range.", RED);
        UseResult::Cancelled
    }
}

fn cast_confuse(_inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) -> UseResult {
    // let monster_id = target_monster(CONFUSE_RANGE, objects, tcod);
    let monster_id = target_monster(tcod, game, objects, Some(CONFUSE_RANGE as f32));
    if let Some(monster_id) = monster_id {
        let old_ai = objects[monster_id].ai.take().unwrap_or(Ai::Basic);
        objects[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: CONFUSE_NUM_TURNS
        });
        game.messages.add(format!("The eyes of the {} glaze over, and it starts to stumble around.", objects[monster_id].name), LIGHT_GREEN);
        UseResult::UsedUp
    } else {
        game.messages.add("No enemy is close enough to strike", RED);
        UseResult::Cancelled
    }
}

fn cast_fireball(_inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) -> UseResult {
    game.messages.add("Left-click a tile to cast a fireball at it; right-click or Esc to cancel", LIGHT_CYAN);
    let (x, y) = match target_tile(tcod, game, objects, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled,
    };
    game.messages.add(format!("The fireball explodes, buring everything within {} tiles.", FIREBALL_RADIUS), ORANGE);
    let mut xp_to_gain = 0;
    for (id, obj) in objects.iter_mut().enumerate() {
        if obj.distance(x, y) <= FIREBALL_RADIUS as f32 && obj.fighter.is_some() {
            game.messages.add(format!("The {} gets burned for {} hit points.", obj.name, FIREBALL_DAMAGE), ORANGE);
            if let Some(xp) = obj.take_damage(FIREBALL_DAMAGE, game) {
                if id != PLAYER {
                    xp_to_gain += xp;
                }
            }
        }
    }
    // TODO: determine attacker rather than awarding to player
    objects[PLAYER].fighter.as_mut().unwrap().xp += xp_to_gain;
    UseResult::UsedUp
}

fn examine_artifact(inventory_id: usize, _tcod: &mut Tcod, game: &mut Game, _objects: &mut [Object]) -> UseResult {
    match &game.inventory[inventory_id].item {
        Some(item) => {
            match item {
                Item::Artifact {name, value} => {
                    game.messages.add(format!("This artifact is named {} and has a value of {}", name, value), GOLD);
                    return UseResult::UsedAndKept
                },
                _ => {
                    game.messages.add("Error: examine_artifact was called with an item that was not an artifact", DARK_RED);
                    return UseResult::Cancelled
                }
            };
        },
        None => {
            game.messages.add("Error: examine_artifact was called when there was no item", DARK_RED);
            return UseResult::Cancelled
        }
    };
}

fn toggle_equipment(inventory_id: usize, _tcod: &mut Tcod, game: &mut Game, _objects: &mut [Object]) -> UseResult {
    let equipment = match game.inventory[inventory_id].equipment {
        Some(equipment) => equipment,
        None => return UseResult::Cancelled,
    };
    if let Some(current_equipment_id) = get_equipped_id_in_slot(equipment.slot, &game.inventory) {
        game.inventory[current_equipment_id].unequip(&mut game.messages);
    }
    if equipment.equipped {
        game.inventory[inventory_id].unequip(&mut game.messages);
    } else {
        game.inventory[inventory_id].equip(&mut game.messages);
    }
    UseResult::UsedAndKept
}

fn get_equipped_id_in_slot(slot: Slot, inventory: &[Object]) -> Option<usize> {
    for (inventory_id, item) in inventory.iter().enumerate() {
        if item.equipment.as_ref().map_or(false, |e| e.equipped && e.slot == slot) {
            return Some(inventory_id)
        }
    }
    None
}

fn closest_monster(tcod: &Tcod, objects: &[Object], max_range: i32) -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range +1) as f32;

    for (id, object) in objects.iter().enumerate() {
        if id != PLAYER && object.fighter.is_some() && object.ai.is_some() && tcod.fov.is_in_fov(object.x, object.y) {
            let dist = objects[PLAYER].distance_to(object);
            if dist < closest_dist {
                closest_enemy = Some(id);
                closest_dist = dist;
            }
        }
    }
    closest_enemy
}

fn target_monster(
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &[Object],
    max_range: Option<f32>
) -> Option<usize> {
    loop {
        match target_tile(tcod, game, objects, max_range) {
            Some((x,y)) =>
            for (id, obj) in objects.iter().enumerate() {
                if obj.pos() == (x, y) && obj.fighter.is_some() && id != PLAYER {
                    return Some(id)
                }
            },
            None => return None
        }
    }
}

fn target_tile(
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &[Object],
    max_range: Option<f32>
) -> Option<(i32, i32)> {
    use tcod::input::KeyCode::Escape;
    loop {
        tcod.root.flush();
        let event = input::check_for_event(input::KEY_PRESS | input::MOUSE).map(|e| e.1);
        match event {
            Some(Event::Mouse(m)) => tcod.mouse = m,
            Some(Event::Key(k)) => tcod.key = k,
            None => tcod.key = Default::default()
        }
        render_all(tcod, game, objects, false);
        let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);

        let in_fov = (x < MAP_WIDTH) && (y < MAP_HEIGHT) && tcod.fov.is_in_fov(x, y);
        let in_range = max_range.map_or(true, |range| objects[PLAYER].distance(x, y) <= range);
        if tcod.mouse.lbutton_pressed && in_fov && in_range {
            return Some((x, y))
        } 
        if tcod.mouse.rbutton_pressed || tcod.key.code == Escape {
            return None;
        }
    }
}

#[derive(Serialize, Deserialize)]
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
        .filter(|o| 
            tcod.fov.is_in_fov(o.x, o.y)                                            // is in fov
            || (o.always_visible && game.map[o.x as usize][o.y as usize].explored)  // is always visible and has been explored
        )
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
    let max_hp = objects[PLAYER].max_hp(game);
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
    // display game level
    tcod.panel.print_ex(1, 3, BackgroundFlag::None, TextAlignment::Left, format!("Level {}", game.dungeon_level));
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

fn handle_keys(tcod: &mut Tcod, objects: &mut Vec<Object>, game: &mut Game) -> PlayerAction {
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
        (Key { code: Up, .. }, _, true ) | (Key { code: NumPad8, .. }, _, true ) => {
            player_move_or_attack(0, -1, game, objects);
            TookTurn
        },
        (Key { code: Down, .. }, _, true ) | (Key { code: NumPad2, .. }, _, true ) => {
            player_move_or_attack(0, 1, game, objects);
            TookTurn
        },
        (Key { code: Left, .. }, _, true ) | (Key { code: NumPad4, .. }, _, true ) => {
            player_move_or_attack(-1, 0, game, objects);
            TookTurn
        },
        (Key { code: Right, .. }, _, true ) | (Key { code: NumPad6, .. }, _, true ) => {
            player_move_or_attack(1, 0, game, objects);
            TookTurn
        },
        (Key { code: Home, .. }, _, true ) | (Key { code: NumPad7, .. }, _, true ) => {
            player_move_or_attack(-1, -1, game, objects);
            TookTurn
        },
        (Key { code: PageUp, .. }, _, true ) | (Key { code: NumPad9, .. }, _, true ) => {
            player_move_or_attack(1, -1, game, objects);
            TookTurn
        },
        (Key { code: End, .. }, _, true ) | (Key { code: NumPad1, .. }, _, true ) => {
            player_move_or_attack(-1, 1, game, objects);
            TookTurn
        },
        (Key { code: PageDown, .. }, _, true ) | (Key { code: NumPad3, .. }, _, true ) => {
            player_move_or_attack(1, 1, game, objects);
            TookTurn
        },
        (Key { code: NumPad5, .. }, _, true ) | (Key { code: Text, .. }, ".", true ) => {
            TookTurn
        },
        (Key { code: Text, .. }, "g", true ) => {
            let item_id = objects.iter().position(|object| object.pos() == objects[PLAYER].pos() && object.item.is_some());
            if let Some(item_id) = item_id {
                pick_item_up(item_id, game, objects);
            }
            DidntTakeTurn
        },
        (Key { code: Text, .. }, "i", true ) => {
            let inventory_selection = inventory_menu(
                &game.inventory, "Select an item to use by pressing the matching key, or any other to cancel\n",  &mut tcod.root
            );
            if let Some(inventory_selection) = inventory_selection {
                use_item(inventory_selection, tcod, game, objects);
            }
            DidntTakeTurn
        },
        (Key {code: Text, ..}, "d", true ) => {
            let inventory_selection = inventory_menu(
                &game.inventory, "Select an item you want to drop by pressing the matching key, or any other to cancel\n", &mut tcod.root
            );
            if let Some(inventory_selection) = inventory_selection {
                drop_item(inventory_selection, game, objects);
            }
            DidntTakeTurn
        },
        (Key {code: Text, ..}, "<", true) => {
            let player_on_stairs = objects
            .iter()
            .any(|object| object.pos() == objects[PLAYER].pos() && object.name == "stairs");
            if player_on_stairs {
                next_level(tcod, game, objects);
            }
            DidntTakeTurn
        },
        (Key {code: Text, ..}, "c", true) => {
            let player = &objects[PLAYER];
            let level = player.level;
            let level_up_xp = LEVEL_UP_BASE + level * LEVEL_UP_FACTOR;
            if let Some(fighter) = player.fighter.as_ref() {
                let msg = format!(
                    "Player stats: \n Level: {}\nExperience: {}\nExperience to level up: {}\n\nMaximum HP: {}\nAttack: {}\nbase_Defense: {}",
                    level, fighter.xp, level_up_xp, player.max_hp(game), player.power(game), player.defense(game)
                );
                msgbox(&msg, STATS_SCREEN_WIDTH, &mut tcod.root);
                
            }
            DidntTakeTurn
        }
        _ => DidntTakeTurn // everything else
    }

}

fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
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

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
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

fn msgbox(text: &str, width: i32, root: &mut Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
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

fn new_game(tcod: &mut Tcod) -> (Game, Vec<Object>) {
    let mut player = Object::new(0, 0, '@', WHITE, "player", true);
    player.alive = true;
    player.fighter = Some(Fighter {
        base_max_hp: 30,
        hp: 30,
        base_defense: 2,
        base_power: 3,
        xp: 200,
        on_death: DeathCallback::Player
    });
    
    let mut objects = vec![player];

    let mut game = Game {
        map: make_map(&mut objects, 1),
        messages: Messages::new(),
        inventory: vec![],
        dungeon_level: 1,
    };

    let mut dagger = Object::new(0, 0, '-', SKY, "dagger", false);
    dagger.item = Some(Item::Sword);
    dagger.equipment = Some(Equipment {
        equipped: true, slot: Slot::LeftHand, max_hp_bonus: 0, defense_bonus: 0, power_bonus: 2
    });
    game.inventory.push(dagger);

    initialize_fov(tcod, &game.map);

    game.messages.add(
        "Welcome to the Halls of Ruzt - there's no time to change your mind...", RED
    );

    (game, objects)
}

fn initialize_fov(tcod: &mut Tcod, map: &Map) {
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

fn run_game_loop(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<Object>) {
    // for FOV recompute by setting player position to a weird value
    let mut previous_player_position = (-1, -1);

    while !tcod.root.window_closed() {
        // clear offscreen console before drawing anything
        tcod.con.clear();

        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => tcod.key = k,
            _ => tcod.key = Default::default(),
        }
        
        let fov_recompute = previous_player_position != (objects[PLAYER].pos());
        
        render_all(tcod, game, &objects, fov_recompute);
        
        tcod.root.flush();

        level_up(tcod, game, objects);
        
        previous_player_position = objects[PLAYER].pos();
        let player_action = handle_keys(tcod, objects, game);
        if player_action == PlayerAction::Exit {
            save_game(game, objects).unwrap();
            break;
        }
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, &tcod, game, objects)
                }
            }
        }
    }
}

fn main_menu(tcod: &mut Tcod) {
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
                let (mut game, mut objects) = new_game(tcod);
                run_game_loop(tcod, &mut game, &mut objects);
            }
            Some(1) => {
                match load_game() {
                    Ok((mut game, mut objects)) => {
                        initialize_fov(tcod, &game.map);
                        run_game_loop(tcod, &mut game, &mut objects);
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

// return type is a result, which can either be a success, or a type that implements the error type.
fn save_game(game: &mut Game, objects: &[Object]) -> Result<(), Box<dyn Error>> {
    let save_data = serde_json::to_string(&(game, objects))?;       // the ? gets the success value, or returns immediately with the error type
    let mut file = File::create("savegame")?;
    file.write_all(save_data.as_bytes())?;
    Ok(())
}

fn load_game() -> Result<(Game, Vec<Object>), Box<dyn Error>> {
    let mut json_save_state = String::new();
    let mut file = File::open("savegame")?;
    file.read_to_string(&mut json_save_state)?;
    let result = serde_json::from_str::<(Game, Vec<Object>)>(&json_save_state)?;
    Ok(result)
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

    main_menu(&mut tcod);
}