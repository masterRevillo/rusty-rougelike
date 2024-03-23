use tcod::colors::{RED, VIOLET};
use crate::entities::entity::Entity;
use crate::framework::GameFramework;
use crate::game_engine::{FOV_ALGO, FOV_LIGHT_WALLS, GameEngine, PLAYER, TORCH_RADIUS};
use crate::map::mapgen::{from_dungeon_level, LEVEL_TYPE_TRANSITION, make_boss_map, make_map, Map};
use crate::entities::entity_actions::target_tile;
use crate::graphics::render_functions::initialize_fov;

pub fn is_blocked(x: i32, y: i32, map: &Map, entity: &[Entity]) -> bool {
    if map[x as usize][y as usize].blocked {
        return true;
    }
    entity.
        iter()
        .any(|object| object.blocks && object.pos() == (x,y))
}

pub fn next_level(tcod: &mut GameFramework, game: &mut GameEngine) {
    game.messages.add("You rest for a minute and recover your strength", VIOLET);
    let heal_hp = game.entities[PLAYER].max_hp() / 2;
    game.entities[PLAYER].heal(heal_hp);
    game.messages.add("You descend deeper into the dungeon ...", RED);
    game.dungeon_level += 1;
    let dungeon_level = game.dungeon_level;
    game.map = match from_dungeon_level(LEVEL_TYPE_TRANSITION, dungeon_level) {
        0 => make_map(game, dungeon_level),
        1 => make_boss_map(game, dungeon_level),
        _ => make_map(game, dungeon_level),
    };
    initialize_fov(tcod, &game.map);
    tcod.fov.compute_fov(game.entities[PLAYER].x, game.entities[PLAYER].y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO)
}

pub fn closest_monster(tcod: &GameFramework, game: &mut GameEngine, max_range: i32) -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range +1) as f32;

    for (id, object) in game.entities.iter().enumerate() {
        if id != PLAYER && object.fighter.is_some() && object.ai.is_some() && tcod.fov.is_in_fov(object.x, object.y) {
            let dist = game.entities[PLAYER].distance_to(object);
            if dist < closest_dist {
                closest_enemy = Some(id);
            closest_dist = dist;
            }
        }
    }
    closest_enemy
}

pub fn target_monster(
    tcod: &mut GameFramework,
    game: &mut GameEngine,
    max_range: Option<f32>
) -> Option<usize> {
    loop {
        match target_tile(tcod, game, max_range) {
            Some((x,y)) =>
                for (id, obj) in game.entities.iter().enumerate() {
                    if obj.pos() == (x, y) && obj.fighter.is_some() && id != PLAYER {
                        return Some(id)
                    }
                },
            None => return None
        }
    }
}
