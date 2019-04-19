/// this file will hold functions used by a variety of things
use crate::constants::*;
use crate::user_defined::*;
use crate::render::*;
use crate::spells::*;

use tcod::colors::{self};
use std::cmp;

pub fn is_blocked(x: i32, y:i32, map: &Map, objects: &[Object]) -> bool {
    // first test the map tile
    if map[x as usize][y as usize].blocked {
        return true;
    }
    // now check for any blocking objects
    objects.iter().any(|object| {
        object.blocks && object.pos() == (x, y)
    })
}

pub fn from_dungeon_level(table: &[Transition], level: u32) -> u32 {
    table.iter()
        .rev()
        .find(|transition| level >= transition.level)
        .map_or(0, |transition| transition.value)
}

/// Mutably borrow two *separate* elements from the given slice.
/// Panics when the indexs are equal or out of bounds
pub fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index != second_index);
    let split_at_index = cmp::max(first_index, second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}

/// move by the given amount, if the destination is not blocked
pub fn move_by(id: usize, dx: i32, dy: i32, game: &mut Game, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, &game.map, objects){
        objects[id].set_pos(x + dx, y + dy);
    }
}

// move an object towards a position
pub fn move_towards(id: usize, target_x: i32, target_y: i32, game: &mut Game, objects: &mut [Object]) {
    // vector from this object to the target, and distance
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    // normalize it to length 1 (preserving direction), then round it and 
    // convert it to integer so the movement is restricted to the map grid
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, game, objects);
}

pub fn toggle_equipment(inventory_id: usize, _objects: &mut [Object], game: &mut Game, _tcod: &mut Tcod) -> UseResult {
    let equipment = match game.inventory[inventory_id].equipment {
        Some(equipment) => equipment,
        None => return UseResult::Cancelled,
    };
    if equipment.equipped {
        game.inventory[inventory_id].dequip(&mut game.log);
    } else {
        game.inventory[inventory_id].equip(&mut game.log);
    }
    UseResult::UsedAndKept
}

/// find the closes enemy, up to a maximum range, an din the player's FOV
pub fn closest_monster(max_range: i32, objects: &mut [Object], tcod: &Tcod) -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32; // start with (slightly more than) max range
    for (id, object) in objects.iter().enumerate() {
        if (id != PLAYER) && object.fighter.is_some() && object.ai.is_some() &&
            tcod.fov.is_in_fov(object.x, object.y) {
                // calculate the distance between the object and the player
                let dist = objects[PLAYER].distance_to(object);
                if dist < closest_dist {
                    // it's closer, so remember it
                    closest_enemy = Some(id);
                    closest_dist = dist;
                }
            }
    }
    closest_enemy
}

pub fn get_equipped_in_slot(slot: Slot, inventory: &[Object]) -> Option<usize> {
    for (inventory_id, item) in inventory.iter().enumerate() {
        if item.equipment.as_ref().map_or(false, |e| e.equipped && e.slot == slot) {
            return Some(inventory_id)
        }
    }
    None
}

pub fn player_move_or_attack(dx: i32, dy: i32, game: &mut Game, objects: &mut [Object]) {
    // the coordinates the player is moving to/attacking
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;

    // try to find an attackable object there
    let target_id = objects.iter().position(|object| {
        object.fighter.is_some() && object.pos() == (x, y)
    });

    // attack if target found, move otherwise
    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER, target_id, objects);
            player.attack(target, game);
        }
        None => {
            move_by(PLAYER, dx, dy, game, objects);
        }
    }
}

/// add to the player's inventory and remove from the map
pub fn pick_item_up(object_id: usize, objects: &mut Vec<Object>, game: &mut Game) {
    if game.inventory.len() >= 26 {
        game.log.add(format!("Your inventory is full, cannot pick up {}.", objects[object_id].name), colors::RED);
    } else {
        let item = objects.swap_remove(object_id);
        game.log.add(format!("You picked up a {}!", item.name), colors::GREEN);
        let index = game.inventory.len();
        let slot = item.equipment.map(|e| e.slot);
        game.inventory.push(item);

        // automatically equip, if the corresponding equipment slot is unused
        if let Some(slot) = slot {
            if get_equipped_in_slot(slot, &game.inventory).is_none() {
                game.inventory[index].equip(&mut game.log);
            }
        }
    }
}

pub fn use_item(inventory_id: usize, objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) {
    use Item::*;
    // just call the 'use_function' if it's defined
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use: fn(usize, &mut [Object], &mut Game, &mut Tcod) -> UseResult = match item {
            Heal => cast_heal,
            Lightning => cast_lightning,
            Confuse => cast_confuse,
            Fireball => cast_fireball,
            Sword => toggle_equipment,
            Shield => toggle_equipment,
        };
        match on_use(inventory_id, objects, game, tcod) {
            UseResult::UsedUp => {
                // destroy after use, unless it was cancelled for some reason
                game.inventory.remove(inventory_id);
            }
            UseResult::UsedAndKept => {}, // do nothing
            UseResult::Cancelled => {
                game.log.add("Cancelled", colors::WHITE);
            }
        }
    } else {
        game.log.add(format!("The {} cannot be used.", game.inventory[inventory_id].name), colors::WHITE);
    }
}

pub fn drop_item(inventory_id: usize,
            game: &mut Game,
            objects: &mut Vec<Object>) {
    let mut item = game.inventory.remove(inventory_id);
    if item.equipment.is_some() {
        item.dequip(&mut game.log);
    }
    item.set_pos(objects[PLAYER].x, objects[PLAYER].y);
    game.log.add(format!("You dropped a {}.", item.name), colors::YELLOW);
    objects.push(item);
}

pub fn level_up(objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) {
    let player = &mut objects[PLAYER];
    let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
    // see if the player's experience is enought to level up
    if player.fighter.as_ref().map_or(0, |f| f.xp) >= level_up_xp {
        // it is! level up
        player.level += 1;
        game.log.add(format!("Your battle skills grow stringer! You reached level {}!", player.level), colors::YELLOW);
        // increase player's stats
        let fighter = player.fighter.as_mut().unwrap();
        let mut choice = None;
        while choice.is_none() {
            // keep asking until a choice is made
            choice = menu(
                "Level up! Choose a stat to raise:\n",
                &[format!("Constitution (+20 HP, from {})", fighter.base_max_hp),
                format!("Strength (+1 attack, from {}", fighter.base_power),
                format!("Agility (+1 defense, from {}", fighter.base_defense)],
                LEVEL_SCREEN_WIDTH, 
                &mut tcod.root
            );
        };
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
            _ => unreachable!(),
        }
    }
}

