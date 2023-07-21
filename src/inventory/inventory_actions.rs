use std::borrow::BorrowMut;
use tcod::colors::{DARK_RED, GOLD, LIGHT_BLUE, LIGHT_CYAN, LIGHT_GREEN, ORANGE, RED, WHITE, YELLOW};
use crate::framework::Tcod;
use crate::game_engine::{GameEngine, PLAYER};
use crate::entities::entity::Entity;
use crate::entities::entity_actions::target_tile;
use crate::entities::slot::Slot;
use crate::items::item::*;
use crate::map::map_functions::{closest_monster, target_monster};
use crate::util::ai::Ai;

pub fn use_item(inventory_id: usize, tcod: &mut Tcod, game: &mut GameEngine) {
    use Item::*;
    if let Some(item) = &game.entities[PLAYER].inventory[inventory_id].item {
        let on_use = match item {
            Heal => cast_heal,
            Lightning => cast_lightning,
            Confuse => cast_confuse,
            Fireball => cast_fireball,
            Artifact{name: _, value: _} => examine_artifact,
            Sword => toggle_equipment,
            Shield => toggle_equipment,
        };
        match on_use(inventory_id, tcod, game) {
            UseResult::UsedUp => {
                game.entities[PLAYER].inventory.remove(inventory_id);
            }
            UseResult::UsedAndKept => {}
            UseResult::Cancelled => {
                game.messages.add("Cancelled", WHITE);
            }
        }
    } else {
        game.messages.add(format!("The {} cannot be used.", game.entities[PLAYER].inventory[inventory_id].name), WHITE);
    }
}

pub fn drop_item(inventory_id: usize, game: &mut GameEngine) {
    //TODO dont default to players inventory
    let mut item = game.entities[PLAYER].inventory.remove(inventory_id);
    if item.equipment.is_some() {
        item.unequip(&mut game.messages);
    }
    item.set_pos(game.entities[PLAYER].x, game.entities[PLAYER].y);
    game.messages.add(format!("You dropped the {}.", item.name), YELLOW);
    game.entities.push(item);
}

pub fn cast_heal(
    _inventory_id: usize,
    _tcod: &mut Tcod,
    game: &mut GameEngine
) -> UseResult {
    let player = &mut game.entities[PLAYER];
    if let Some(fighter) = player.fighter {
        if fighter.hp == player.max_hp() {
            // game.messages.add("You're already at full health. ", RED);
            return UseResult::Cancelled;
        }
        // game.messages.add("Your wounds feel a bit better", LIGHT_VIOLET);
        player.heal(HEAL_AMOUNT);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

pub fn cast_lightning(
    _inventory_id: usize,
    tcod: &mut Tcod,
    game: &mut GameEngine,
) -> UseResult {

    let monster_id = closest_monster(tcod, game, LIGHTNING_RANGE);
    let entities: &mut Vec<Entity> = game.entities.borrow_mut();
    let event_bus = game.event_bus.borrow_mut();
    let messages = game.messages.borrow_mut();
    if let Some(monster_id) = monster_id {
        game.messages.add(
            format!("A lightning bolt strikes the {}! It deals {} points of damage.", entities[monster_id].name, LIGHTNING_DAMAGE),
            LIGHT_BLUE
        );
        if let Some(xp) = entities[monster_id].take_damage(LIGHTNING_DAMAGE, event_bus) {
            // TODO: determine attacker and award xp to them, not automatically to player
            entities[PLAYER].fighter.as_mut().unwrap().xp += xp;
        }
        UseResult::UsedUp
    } else {
        messages.add("No enemies are within range.", RED);
        UseResult::Cancelled
    }
}

pub fn cast_confuse(_inventory_id: usize, tcod: &mut Tcod, game: &mut GameEngine) -> UseResult {
    // let monster_id = target_monster(CONFUSE_RANGE, objects, tcod);
    let monster_id = target_monster(tcod, game, Some(CONFUSE_RANGE as f32));
    if let Some(monster_id) = monster_id {
        let old_ai = game.entities[monster_id].ai.take().unwrap_or(Ai::Basic);
        game.entities[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: CONFUSE_NUM_TURNS
        });
        game.messages.add(format!("The eyes of the {} glaze over, and it starts to stumble around.", game.entities[monster_id].name), LIGHT_GREEN);
        UseResult::UsedUp
    } else {
        game.messages.add("No enemy is close enough to strike", RED);
        UseResult::Cancelled
    }
}

pub fn cast_fireball(_inventory_id: usize, tcod: &mut Tcod, game: &mut GameEngine) -> UseResult {
    game.messages.add("Left-click a tile to cast a fireball at it; right-click or Esc to cancel", LIGHT_CYAN);
    let (x, y) = match target_tile(tcod, game, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled,
    };
    let entities: &mut Vec<Entity> = game.entities.borrow_mut();
    let event_bus = game.event_bus.borrow_mut();
    let messages = game.messages.borrow_mut();
    messages.add(format!("The fireball explodes, burning everything within {} tiles.", FIREBALL_RADIUS), ORANGE);
    let mut xp_to_gain = 0;
    for (id, obj) in entities.iter_mut().enumerate() {
        if obj.distance(x, y) <= FIREBALL_RADIUS as f32 && obj.fighter.is_some() {
            game.messages.add(format!("The {} gets burned for {} hit points.", obj.name, FIREBALL_DAMAGE), ORANGE);
            if let Some(xp) = obj.take_damage(FIREBALL_DAMAGE, event_bus) {
                if id != PLAYER {
                    xp_to_gain += xp;
                }
            }
        }
    }
    // TODO: determine attacker rather than awarding to player
    entities[PLAYER].fighter.as_mut().unwrap().xp += xp_to_gain;
    UseResult::UsedUp
}

pub fn examine_artifact(inventory_id: usize, _tcod: &mut Tcod, game: &mut GameEngine) -> UseResult {
    //TODO: dont default to player inventory
    match &game.entities[PLAYER].inventory[inventory_id].item {
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

pub fn toggle_equipment(inventory_id: usize, _tcod: &mut Tcod, game: &mut GameEngine) -> UseResult {
    //TODO: dont default to player inventory
    let messages = game.messages.borrow_mut();
    let player = game.entities[PLAYER].borrow_mut();
    let equipment = match player.inventory[inventory_id].equipment {
        Some(equipment) => equipment,
        None => return UseResult::Cancelled,
    };
    if let Some(current_equipment_id) = get_equipped_id_in_slot(equipment.slot, &player.inventory) {
        player.inventory[current_equipment_id].unequip(messages);
    }
    if equipment.equipped {
        player.inventory[inventory_id].unequip(messages);
    } else {
        player.inventory[inventory_id].equip(messages);
    }
    UseResult::UsedAndKept
}

pub fn get_equipped_id_in_slot(slot: Slot, inventory: &[Entity]) -> Option<usize> {
    for (inventory_id, item) in inventory.iter().enumerate() {
        if item.equipment.as_ref().map_or(false, |e| e.equipped && e.slot == slot) {
            return Some(inventory_id)
        }
    }
    None
}