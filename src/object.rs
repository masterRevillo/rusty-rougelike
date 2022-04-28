use core::option::Option;
use core::option::Option::{None, Some};
use tcod::colors::{Color, LIGHT_GREEN, LIGHT_YELLOW, RED, WHITE};
use tcod::console::{BackgroundFlag, Console};
use crate::{Ai, Equipment, EventType, Fighter, Game, GameEvent, Item, Messages};
use serde::{Deserialize, Serialize};

/// This is a generic object: the player, a monster, an item, the stairs...
/// It's always represented by a character on screen.
#[derive(Debug, Serialize, Deserialize)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub char: char,
    pub color: Color,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<Fighter>,
    pub ai: Option<Ai>,
    pub item: Option<Item>,
    pub always_visible: bool,
    pub level: i32,
    pub equipment: Option<Equipment>,
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
            game.add_event(GameEvent::from_type(EventType::PlayerAttack));
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
