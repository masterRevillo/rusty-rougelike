use std::borrow::BorrowMut;
use std::cmp;
use rand::Rng;
use tcod::colors::{DARK_CRIMSON, DARK_ORANGE, DARKER_AMBER, DARKER_AZURE, DESATURATED_GREEN, GOLD, LIGHT_RED, LIGHT_YELLOW, LIGHTEST_SEPIA, LIGHTEST_YELLOW, SKY, VIOLET, WHITE};
use crate::{Entity, from_dungeon_level, GameEngine, IndependentSample, is_blocked, Item, PLAYER, Transition, Weighted, WeightedChoice};
use crate::entity::equipment::Equipment;
use crate::entity::fighter::Fighter;
use crate::entity::slot::Slot;
use crate::map::tile::Tile;
use crate::util::ai::Ai;
use crate::util::death_callback::DeathCallback;
use crate::util::namegen;

pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 68;

//parameters for dungeon generator
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 32;

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
    Transition{ level: 3, value: 5 },
    Transition{ level: 5, value: 10 },
    Transition{ level: 7, value: 30 },
];

const SPECTRE_CHANCE_TRANSITION: &[Transition] = &[
    Transition{ level: 6, value: 10 },
    Transition{ level: 8, value: 30 },
    Transition{ level: 10, value: 70 },
];

const MAX_ITEMS_TRANSITION: &[Transition] = &[
    Transition{ level: 1, value: 1 },
    Transition{ level: 4, value: 2 },
];

const ROOM_OVERLAP_TRANSITION: &[Transition] = &[
    Transition{ level: 3, value: 1 },
];

pub const LEVEL_TYPE_TRANSITION: &[Transition] = &[
    Transition{ level: 1, value: 0 },
    Transition{ level: 2, value: 1 },
    Transition{ level: 3, value: 0 },
    Transition{ level: 10, value: 2 },
];

pub type Map = Vec<Vec<Tile>>;

pub fn in_map_bounds(x: i32, y: i32) -> bool {
    0 <= x && x < MAP_WIDTH && 0 <= y && y < MAP_HEIGHT
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
            map[x as usize][y as usize] = Tile::ground();
        }
    }
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::ground();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::ground();
    }
}

pub fn make_map(game :&mut GameEngine, level: u32) -> Map {
    // TODO: better map initialization
    // this is kinda dumb... because the vec macro only calls the constructor once, it was using the same
    // rng value ever time. So to fix this, I init everything to empty, then go back through and init with a
    // new map tile. It works, and since the surface chars are only decorative now, I guess its fine
    let entities: &mut Vec<Entity> = game.entities.borrow_mut();

    let mut map = vec![vec![Tile::ground(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];    // vec! is a shorthand macro that initializes the Vec and fills it with the specified value
    for x in 0..MAP_WIDTH as usize {
        for y in 0..MAP_HEIGHT as usize {
            map[x][y] = Tile::wall();
        }
    }

    // the syntax is vec![value_to_fill, number_of_entries]

    let mut rooms: std::vec::Vec<Rect> = vec![];

    // TODO: this will break if the player is not the first object in the list... that should change
    // this compares the pointers to both objects and verifies that they are the same value
    assert_eq!(&entities[PLAYER] as *const _, &entities[0] as *const _);
    entities.truncate(1);

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
            place_objects(new_room, &map, entities, level);

            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() {
                entities[PLAYER].set_pos(new_x, new_y);
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
    let mut stairs = Entity::new(last_room_x, last_room_y, '<', WHITE, "stairs", false);
    stairs.always_visible = true;
    entities.push(stairs);
    map
}

pub fn make_boss_map(game: &mut GameEngine, _level: u32) -> Map {
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
    assert_eq!(&game.entities[PLAYER] as *const _, &game.entities[0] as *const _);
    game.entities.truncate(1);

    let boss_room = Rect::new(0, 0, MAP_WIDTH - 2, MAP_HEIGHT - 2);
    create_room(boss_room, &mut map);

    let (center_x, center_y) = boss_room.center();

    game.entities[PLAYER].set_pos(center_x, 3);
    let mut boss = Entity::new(center_x, center_y, 'B', DARK_CRIMSON, "Boss", true);
    boss.fighter = Some(Fighter {base_max_hp: 1, hp: 1, base_defense: 1, base_power: 1, xp: 1000, on_death: DeathCallback::Boss });
    // boss.fighter = Some(Fighter {base_max_hp: 50, hp: 50, base_defense: 8, base_power: 11, xp: 1000, on_death: DeathCallback::Monster });
    boss.ai = Some(Ai::Basic);
    game.entities.push(boss);
    map
}

fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Entity>, level: u32) {
    let max_monsters = from_dungeon_level(MAX_MONSTERS_TRANSITION, level);

    let num_monsters = rand::thread_rng().gen_range(0, max_monsters + 1);

    let troll_chance = from_dungeon_level(TROLL_CHANCE_TRANSITION, level);
    let skeleton_chance = from_dungeon_level(SKELETON_CHANCE_TRANSITION, level);
    let spectre_chance = from_dungeon_level(SPECTRE_CHANCE_TRANSITION, level);
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
        },
        Weighted {
            weight: spectre_chance,
            item: "spectre"
        }
    ];
    let monster_choice = WeightedChoice::new(monster_chances);

    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {
            let mut monster = match monster_choice.ind_sample(&mut rand::thread_rng()) {
                "skeleton" => {
                    let mut skeleton = Entity::new(x, y, 's', LIGHTEST_SEPIA, "Skeleton", true);
                    skeleton.fighter = Some(Fighter {base_max_hp: 25, hp: 25, base_defense: 1, base_power: 6, xp: 200, on_death: DeathCallback::Monster });
                    skeleton.ai = Some(Ai::Basic);
                    skeleton
                },
                "troll" => {
                    let mut troll = Entity::new(x, y, 'T', DARKER_AMBER, "Troll", true);
                    troll.fighter = Some(Fighter {base_max_hp: 30, hp: 30, base_defense: 2, base_power: 4, xp: 100, on_death: DeathCallback::Monster });
                    troll.ai = Some(Ai::Basic);
                    troll
                },
                "orc" => {
                    let mut orc = Entity::new(x, y, 'o', DESATURATED_GREEN, "Orc", true);
                    orc.fighter = Some(Fighter {base_max_hp: 10, hp: 10, base_defense: 0, base_power: 3, xp: 35, on_death: DeathCallback::Monster });
                    orc.ai = Some(Ai::Basic);
                    orc
                },
                "spectre" => {
                    let mut orc = Entity::new(x, y, 'o', DARKER_AZURE, "Spectre", true);
                    orc.fighter = Some(Fighter {base_max_hp: 43, hp: 43, base_defense: 4, base_power: 9, xp: 250, on_death: DeathCallback::Monster });
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
                    let mut object = Entity::new(x, y, '!', VIOLET, "health potion", false);
                    object.item = Some(Item::Heal);
                    object
                },
                Item::Lightning => {
                    let mut object = Entity::new(x, y, '#', LIGHT_YELLOW, "scroll of lightning bolt", false);
                    object.item = Some(Item::Lightning);
                    object
                },
                Item::Fireball => {
                    let mut object = Entity::new(x, y, '#', LIGHT_RED, "scroll of firball", false);
                    object.item = Some(Item::Fireball);
                    object
                },
                Item::Confuse => {
                    let mut object = Entity::new(x, y, '#', LIGHTEST_YELLOW, "scroll of confusion", false);
                    object.item = Some(Item::Confuse);
                    object
                },
                Item::Artifact{name: _, value: _} => {
                    let mut object = Entity::new(x, y, '{', GOLD, "artifact", false);
                    object.item = Some(
                        Item::Artifact{
                            name: namegen::generate_artifact_name(2,7),
                            value: 250 * rand::thread_rng().gen_range(1, 30)
                        }
                    );
                    object
                },
                Item::Sword => {
                    let mut object = Entity::new(x, y, '/', SKY, "sword", false);
                    object.item = Some(Item::Sword);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::RightHand, power_bonus: 3, defense_bonus: 0, max_hp_bonus: 0});
                    object
                },
                Item::Shield => {
                    let mut object = Entity::new(x, y, '[', DARK_ORANGE, "shield", false);
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