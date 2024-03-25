use core::option::Option;
use core::option::Option::{None, Some};
use std::collections::HashMap;
use bracket_lib::color::{LIGHT_GREEN, LIGHT_YELLOW, RED, RGB, RGBA};
use bracket_lib::prelude::{BTerm, to_cp437};
use bracket_lib::terminal::Console;
// use tcod::colors::{Color, LIGHT_GREEN, LIGHT_YELLOW, RED};
// use tcod::console::{BackgroundFlag, Console};
use serde::{Deserialize, Serialize};
use crate::Messages;
use crate::entities::equipment::Equipment;
use crate::entities::fighter::Fighter;
use crate::events::game_event_processing::{EventBus, EventData, EventType, GameEvent};
use crate::graphics::camera::Camera;
use crate::items::item::Item;
use crate::util::ai::Ai;

/// This is a generic object: the player, a monster, an item, the stairs...
/// It's always represented by a character on screen.
#[derive(Debug, Serialize, Deserialize)]
pub struct Entity {
    pub x: i32,
    pub y: i32,
    pub char: char,
    pub color: RGB,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<Fighter>,
    pub ai: Option<Ai>,
    pub item: Option<Item>,
    pub always_visible: bool,
    pub level: i32,
    pub equipment: Option<Equipment>,
    pub inventory: Vec<Entity>,
}

impl Entity {
    pub fn new(x: i32, y: i32, char: char, color: RGB, name: &str, blocks: bool) -> Self {
        Entity {
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
            inventory: vec![]
        }
    }

    // draw self onto given console
    pub fn draw(&self, con: &mut BTerm, camera: &mut Camera) {             // dyn: Console is a "trait", not a struct. dyn is basically used to announce that its a trait
        let (x_in_camera, y_in_camera) = camera.get_pos_in_camera(self.x, self.y);
        // con.set_default_foreground(self.color);                  // pointers to traits are double the size of pointers to structs, so there some implications with using it
        if camera.in_bounds(x_in_camera, y_in_camera) {
            con.set(
                x_in_camera,
                y_in_camera,
                RGBA::from(self.color),
                RGBA::from(self.color),
                to_cp437(self.char),
            )
        }
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn distance_to(&self, other: &Entity) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }

    pub fn take_damage(&mut self, damage: i32, event_bus: &mut EventBus) -> Option<i32>{
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }
        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, event_bus);
                return Some(fighter.xp);
            }
        }
        None
    }

    pub fn attack(&mut self, target: &mut Entity, event_bus: &mut EventBus) {
        let damage = self.power() - target.defense();
        let mut killed = false;
        if damage > 0 {
            // game.messages.add(format!("{} attacks {} for {} hit points", self.name, target.name, damage), WHITE);
            if let Some(xp) = target.take_damage(damage, event_bus) {
                self.fighter.as_mut().unwrap().xp += xp;
                killed = true;
            }
        } else {
            // game.messages.add(format!("{} attacks {}, but it has no effect", self.name, target.name), WHITE);
        }
        event_bus.add_event(GameEvent::from_type_with_data(
            EventType::EntityAttacked,
            HashMap::from([
                ("target_name".to_string(), EventData::String(target.name.clone())),
                ("target_pos".to_string(), EventData::TupleI32I32(target.pos())),
                ("attacker_name".to_string(), EventData::String(self.name.clone())),
                ("attacker_pos".to_string(), EventData::TupleI32I32(self.pos())),
                ("damage".to_string(), EventData::I32(damage)),
                ("killed".to_string(), EventData::Boolean(killed)),
            ])
        ))
    }

    pub fn heal(&mut self, amount: i32) {
        let max_hp = self.max_hp();
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
            messages.add(format!("Cannot equip {:?} because it's not an Item.", self ), RGB::from(RED));
            return;
        };
        if let Some(ref mut equipment) = self.equipment {
            if !equipment.equipped {
                equipment.equipped = true;
                messages.add(format!("Equipped {} on {}.", self.name, equipment.slot), RGB::from(LIGHT_GREEN));
            }
        } else {
            messages.add(format!("Cannot equip {:?} because it's not an Equipment.", self ), RGB::from(RED));
        }
    }

    pub fn unequip(&mut self, messages: &mut Messages) {
        if self.item.is_none() {
            messages.add(format!("Cannot unequip {:?} because it's not an Item.", self ), RGB::from(RED));
            return;
        }
        if let Some(ref mut equipment) = self.equipment {
            if equipment.equipped {
                messages.add(format!("Unequipped {} from {}.", self.name, equipment.slot), RGB::from(LIGHT_YELLOW));
            }
        } else {
            messages.add(format!("Cannot unequip {:?} because it's not an Equipment.", self ), RGB::from(RED));
        }
    }

    pub fn power(&self) -> i32 {
        let base_power = self.fighter.map_or(0, |f| f.base_power);
        let bonus: i32 = self.get_all_equipped().iter().map(|e| e.power_bonus).sum();
        base_power + bonus
    }

    pub fn defense(&self) -> i32 {
        let base_defense = self.fighter.map_or(0, |f| f.base_defense);
        let bonus: i32 = self.get_all_equipped().iter().map(|e| e.defense_bonus).sum();
        base_defense + bonus
    }

    pub fn max_hp(&self) -> i32 {
        let base_max_hp = self.fighter.map_or(0, |f| f.base_max_hp);
        let bonus: i32 = self.get_all_equipped().iter().map(|e| e.max_hp_bonus).sum();
        base_max_hp + bonus
    }

    pub fn get_all_equipped(&self) -> Vec<Equipment>{
        self.inventory.iter()
            .filter(|e| e.equipment.map_or(false, |e| e.equipped))
            .map(|e| e.equipment.unwrap())
            .collect()
    }
}
