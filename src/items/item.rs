use serde::{Deserialize, Serialize};

//parameters for items
pub const HEAL_AMOUNT: i32 = 4;
pub const LIGHTNING_DAMAGE: i32 = 40;
pub const LIGHTNING_RANGE: i32 = 5;
pub const CONFUSE_RANGE: i32 = 8;
pub const CONFUSE_NUM_TURNS: i32 = 10;
pub const FIREBALL_RADIUS: i32 = 3;
pub const FIREBALL_DAMAGE: i32 = 12;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Heal,
    Lightning,
    Confuse,
    Fireball,
    Artifact {name: String, value: i32},
    Sword,
    Shield,
}

pub enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
}

