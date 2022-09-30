use serde::{Deserialize, Serialize};
use crate::DeathCallback;

//TODO spilt xp store for player and drop xp into different values
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fighter {
    pub base_max_hp: i32,
    pub hp: i32,
    pub base_defense: i32,
    pub base_power: i32,
    pub xp: i32,
    pub on_death: DeathCallback
}