/// this file will hold everything releated to ai
use crate::user_defined::*;
use crate::constants::*;
use crate::helper::*;

use tcod::colors::{self};
use tcod::map::{Map as FovMap}; // the 'Map as FovMap' section renames the tcod fov map
                                // so that it doesn't conflict with our user defined Map
use rand::Rng;

pub fn ai_take_turn(monster_id: usize, game: &mut Game, objects: &mut [Object], fov_map: &FovMap) {
    // a basic monster takes its turn. If you can see it, it can see you
    use Ai::*;
    if let Some(ai) = objects[monster_id].ai.take() {
        let new_ai = match ai {
            Basic => ai_basic(monster_id, game, objects, fov_map),
            Confused{previous_ai, num_turns} => ai_confused (
                monster_id, game, objects, previous_ai, num_turns)
        };
        objects[monster_id].ai = Some(new_ai);
    }
}

pub fn ai_basic(monster_id: usize, game: &mut Game, objects: &mut [Object], fov_map: &FovMap) -> Ai {
    // a basic monster takes its turn. If you can see it, it can see you
    let (monster_x, monster_y) = objects[monster_id].pos();
    if fov_map.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            // move towards player if far away
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, game, objects);
        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            // close enough, attack! (if the player is still alive)
            let (monster, player) = mut_two(monster_id, PLAYER, objects);
            monster.attack(player, game);
        }
    }
    Ai::Basic
}

pub fn ai_confused(monster_id: usize, game: &mut Game, objects: &mut [Object],
    previous_ai: Box<Ai>, num_turns: i32) -> Ai {
    if num_turns >= 0 {
        // still confused...
        // move in a random direction, and decrease the number of turns confused
        move_by(monster_id,
            rand::thread_rng().gen_range(-1, 2),
            rand::thread_rng().gen_range(-1, 2),
            game, 
            objects);
        Ai::Confused{previous_ai: previous_ai, num_turns: num_turns - 1}
    } else {
        // restore the previous AI (this one will be deleted)
        game.log.add(format!("The {} is no longer confused!",
            objects[monster_id].name), colors::RED);
        *previous_ai
    }
}
