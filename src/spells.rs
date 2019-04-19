/// this file will hold functions for spell-like things
use crate::constants::*;
use crate::helper::*;
use crate::render::*;
use crate::user_defined::*;
use tcod::colors::{self};

pub fn cast_heal(_inventory_id: usize, objects: &mut [Object], game: &mut Game, _tcod: &mut Tcod) -> UseResult {
    // heal the player
    let player = &mut objects[PLAYER];
    if let Some(fighter) = player.fighter {
        if fighter.hp == player.max_hp(game) {
            game.log.add("You are already at full health.", colors::RED);
            return UseResult::Cancelled;
        }
        game.log.add("Your wounds start to feel better!", colors::LIGHT_VIOLET);
        objects[PLAYER].heal(HEAL_AMOUNT, game);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

pub fn cast_lightning(_inventory_id: usize, objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) -> UseResult {
    // find the closest enemy (inside a maximum range) and damage it
    let monster_id = closest_monster(LIGHTNING_RANGE, objects, tcod);
    if let Some(monster_id) = monster_id {
        // zap it
        game.log.add(format!("A lighting bolt strikes the {} with a loud BOOM! \
                The damage is {} hit points.",
                objects[monster_id].name, LIGHTNING_DAMAGE),
            colors::LIGHT_BLUE);
        if let Some(xp) = objects[monster_id].take_damage(LIGHTNING_DAMAGE, game){
            objects[PLAYER].fighter.as_mut().unwrap().xp += xp;
        }
        UseResult::UsedUp
    } else {
        // no enemy found within the maximum range
        game.log.add("No enemy is close enough to strike.", colors::RED);
        UseResult::Cancelled
    }
}

pub fn cast_confuse(_inventory_id: usize, objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) -> UseResult {
    // ask the player for a target to confuse
    game.log.add("Left-click an enemy to confuse it, or right click to cancel.",
            colors::LIGHT_CYAN);
    let monster_id = target_monster(tcod, objects, game, Some(CONFUSE_RANGE as f32));
    if let Some(monster_id) = monster_id {
        let old_ai = objects[monster_id].ai.take().unwrap_or(Ai::Basic);
        // replace the monster's AI with a "confused" one; after
        // some turns it will restore to the old AI
        objects[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: CONFUSE_NUM_TURNS,
        });
        game.log.add(format!("The eyes of the {} look vacant, as it starts to stumble around!",
                objects[monster_id].name),
                colors::LIGHT_GREEN);
        UseResult::UsedUp
    } else {
        // no enemy found within max range
        game.log.add("No enemy is close enough to strike.", colors::RED);
        UseResult::Cancelled
    }
}

pub fn cast_fireball(_inventory_id: usize, objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) -> UseResult {
    // ask the player for a target tile to throw a fireball at
    game.log.add("Left-click a target tile for the fireball, or right-click to cancel.",
        colors::LIGHT_CYAN);
    let (x, y) = match target_tile(tcod, objects, game, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled,
    };
    game.log.add(format!("The fireball exploeds, burning everything within {} tiles!",
            FIREBALL_RADIUS), colors::ORANGE);

    let mut xp_to_gain = 0;
    for (id, obj) in objects.iter_mut().enumerate() {
        if obj.distance(x, y) <= FIREBALL_RADIUS as f32 && obj.fighter.is_some() {
            game.log.add(format!("The {} gets burned for {} hit points.",
                obj.name, FIREBALL_DAMAGE), colors::ORANGE);
            if let Some(xp) = obj.take_damage(FIREBALL_DAMAGE, game) {
                // don't reward the player for burning themself!
                if id != PLAYER {
                    xp_to_gain += xp;
                }
            }
        }
    }
    objects[PLAYER].fighter.as_mut().unwrap().xp += xp_to_gain;
    UseResult::UsedUp
}
