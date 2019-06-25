/// this file holds all functions related to generating the map
use crate::constants::*;
use crate::user_defined::*;
use crate::helper::*;

use std::cmp;
use tcod::colors::{self};
use rand::Rng;
use rand::distributions::{Weighted, WeightedChoice, IndependentSample};

pub fn make_map_debug(objects: &mut Vec<Object>, level: u32) -> Map {
    let mut map = vec![vec![Tile::empty(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    // player is the first element, remove everything else.
    // NOTE: works only when the player is the first object!
    assert_eq!(&objects[PLAYER] as *const _, &objects[0] as *const _);
    objects.truncate(1);

    let player = &mut objects[PLAYER];
    player.set_pos(50, 50);

    // return the map
    map
}

pub fn make_map(objects: &mut Vec<Object>, level: u32) -> Map {
    // fill map with "unblocked" tiles
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    // player is the first element, remove everything else.
    // NOTE: works only when the player is the first object!
    assert_eq!(&objects[PLAYER] as *const _, &objects[0] as *const _);
    objects.truncate(1);

    let mut rooms = vec![];

    for _ in 0..MAX_ROOMS {
        // random width and height
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        // random position without going out of the boundaries of the map
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);

        // run through the other rooms and see if they intersect with this one
        let failed = rooms.iter().any(|other_room| new_room.intersects_with(other_room));

        if !failed {
            // this means there are no intersections, so this room is valid
            
            // paint it to the map's tiles
            create_room(new_room, &mut map);

            // add some content to this room, such as monsters
            place_objects(new_room, &map, objects, level);

            // center coordinates of the new room, will be useful later
            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() {
                // this is the first room, where the player starts at
                // so place them in the center of the room
                let player = &mut objects[PLAYER];
                player.set_pos(new_x, new_y);

            } else {
                // all rooms after the first:
                // connect it to the previous room with a tunnel

                // center coordinates of previous room
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

                // draw a coin (random bool value -- either true or false)
                if rand::random() {
                    // first move horizontally, then vertically
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    // first move vertically, then horizontally
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }
            // finally, append the new room to the list
            rooms.push(new_room);
        }
    }

    // create stairs at the center of thee last room
    let (last_room_x, last_room_y) = rooms[rooms.len() - 1].center();
    let mut stairs = Object::new(last_room_x, last_room_y, '<', "stairs", colors::WHITE, false);
    stairs.always_visible = true;
    objects.push(stairs);

    // return the map and starting position
    map
}

fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

/// take a room and add objects to it (monsters, items, etc)
fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>, level: u32) {

    let max_monsters = from_dungeon_level(&[
        Transition {level: 1, value: 2},
        Transition {level: 4, value: 3},
        Transition {level: 6, value: 5},
    ], level);

    // choose a random number of monsters
    let num_monsters = rand::thread_rng().gen_range(0, max_monsters + 1);

    // monster random table
    let troll_chance = from_dungeon_level(&[
        Transition {level: 3, value: 15},
        Transition {level: 5, value: 30},
        Transition {level: 7, value: 60},
    ], level);

    // monster random table
    let monster_chances = &mut [
        Weighted {weight: 80, item: "orc"},
        Weighted {weight: troll_chance, item: "troll"},
    ];
    let monster_choice = WeightedChoice::new(monster_chances);

    for _ in 0..num_monsters {
        // choose random spot for this monster
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        let mut monster = match monster_choice.ind_sample(&mut rand::thread_rng()) {
            "orc" => {
                let mut orc = Object::new(x, y, 'o', "orc", colors::DESATURATED_GREEN, true);
                orc.fighter = Some(Fighter{base_max_hp: 20, hp: 20, base_defense: 0, base_power: 4, on_death: DeathCallback::Monster, xp: 35});
                orc.ai = Some(Ai::Basic);
                orc
            },
            "troll" => {
                let mut troll = Object::new(x, y, 'T', "troll", colors::DARKER_GREEN, true); // else, a troll
                troll.fighter = Some(Fighter{base_max_hp: 30, hp: 30, base_defense: 2, base_power: 8, on_death: DeathCallback::Monster, xp: 100});
                troll.ai = Some(Ai::Basic);
                troll
            },
            _ => unreachable!(),
        };

        // only place it if the tile is not blocked
        if !is_blocked(x, y, map, objects) {
            monster.alive = true;
            objects.push(monster);
        }
    }

    // max number of items per room
    let max_items = from_dungeon_level(&[
        Transition {level: 1, value: 1},
        Transition {level: 4, value: 2},
    ], level);

    // choose a random number of items
    let num_items = rand::thread_rng().gen_range(0, max_items + 1);

    // item random table
    let item_chances = &mut [
        // healing potion always shows up, even if all other items have 0 chance
        Weighted {weight: 35, item: Item::Heal},
        Weighted {weight: from_dungeon_level(&[Transition {level: 4, value: 25}], level), item: Item::Lightning},
        Weighted {weight: from_dungeon_level(&[Transition {level: 6, value: 25}], level), item: Item::Fireball},
        Weighted {weight: from_dungeon_level(&[Transition {level: 2, value: 10}], level), item: Item::Confuse},
        Weighted {weight: from_dungeon_level(&[Transition {level: 4, value: 5}], level), item: Item::Sword},
        Weighted {weight: from_dungeon_level(&[Transition {level: 8, value: 15}], level), item: Item::Shield},
    ];
    let item_choice = WeightedChoice::new(item_chances);

    for _ in 0..num_items {
        // choose a random spot for this item
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        // only place it if the tile is not blocked
        if !is_blocked(x, y, map, objects) {
            let mut item = match item_choice.ind_sample(&mut rand::thread_rng()) {
                Item::Heal => {
                    let mut object = Object::new(x, y, '!', "healing potion", colors::VIOLET, false);
                    object.item = Some(Item::Heal);
                    object
                },
                Item::Lightning => {
                    let mut object = Object::new(x, y, '#', "scroll of lightning bolt", colors::LIGHT_YELLOW, false);
                    object.item = Some(Item::Lightning);
                    object
                },
                Item::Fireball => {
                    let mut object = Object::new(x, y, '#', "scroll of fireball", colors::LIGHT_YELLOW, false);
                    object.item = Some(Item::Fireball);
                    object
                },
                Item::Confuse => {
                    let mut object = Object::new(x, y, '#', "scroll of confuse", colors::LIGHT_YELLOW, false);
                    object.item = Some(Item::Confuse);
                    object
                },
                Item::Sword => {
                    // create a sword
                    let mut object = Object::new(x, y, '/', "sword", colors::SKY, false);
                    object.item  = Some(Item::Sword);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::RightHand, max_hp_bonus: 0, power_bonus: 3, defense_bonus: 0});
                    object
                },
                Item::Shield => {
                    // create a shield
                    let mut object = Object::new(x, y, '[', "shield", colors::DARKER_ORANGE, false);
                    object.item  = Some(Item::Shield);
                    object.equipment = Some(Equipment{equipped: false, slot: Slot::LeftHand, max_hp_bonus: 0, power_bonus: 0, defense_bonus: 1});
                    object
                }
            };
            item.always_visible = true;
            objects.push(item);
        }
    }

    // max number of torches per room
    let max_torches = 1;
    // choose a random number of torches
    let num_torches = rand::thread_rng().gen_range(0, max_torches + 1);
    for _ in 0..num_torches {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        // only place it if the tile is not blocked
        if !is_blocked(x, y, map, objects) {
            let mut torch = Object::new(x, y, 'i', "torch", colors::DARKEST_ORANGE, false);
            torch.emitter = Some(Emitter{radius: 2, color: colors::DARKEST_ORANGE});
            torch.always_visible = true;
            objects.push(torch);
        }
    }


}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

