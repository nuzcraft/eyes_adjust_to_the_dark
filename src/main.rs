/// Rust/libtcod tutorial, with notes

// tcod is an external crate (and is referenced in the Cargo.toml file)
extern crate tcod;
extern crate rand;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

// constants is a separate file that holds all our constants
mod constants;
use constants::*;
// user_defined is a separate file that holds our user defined structs, types, enums
mod user_defined;
use user_defined::*;
// mapgen is a separate file that holds our map generation code
mod mapgen;
use mapgen::*;
// helper is a separate file that hold functions used by many different sections
mod helper;
use helper::*;
// render is a separate file that holds functions used to render to the screen
mod render;
use render::*;
// ai is a separate file that hold functions related to enemy ai
mod ai;
use ai::*;
mod spells;

use std::io::{Read, Write};
use std::fs::File;
use std::error::Error;
use tcod::console::*;
use tcod::colors::{self};
use tcod::map::{Map as FovMap}; // the 'Map as FovMap' section renames the tcod fov map
                                // so that it doesn't conflict with our user defined Map
use tcod::input::Key;
use tcod::input::{self, Event};

/// main function of the game, starts with initializers, then moves into the main game loop
fn main() {
    
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod) // set up a font. this can be in various formats, must be in the root, next to Cargo.toml
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT) // set the dimensions of the window
        .title("Rust/libtcod tutorial") // name the window
        .init(); // this actually opens the window

    tcod::system::set_fps(LIMIT_FPS); // set the frames per second; limits the refresh rate

    let mut tcod = Tcod {
        root: root,
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT), // create offscreen console for the map
        panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT), // create offscreen console for the gui
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
        mouse: Default::default(),
    };

    main_menu(&mut tcod);
}

/// this function will handle all interactions from the player
/// this will return false if the player wants to continue playing, true to quit
fn handle_keys(key: Key, tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<Object>) -> PlayerAction {

    use tcod::input::KeyCode::*;
    use PlayerAction::*;

    let player_alive = objects[PLAYER].alive;
    match (key, player_alive) {
        (Key {code: Enter, alt: true, ..}, _) => {
            // Alt+Enter: toggle fullscreen
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        },
        (Key {code: Escape, ..}, _) => Exit, // exit game
        // movement keys
        (Key {code: Up, ..}, true) | (Key {code: NumPad8, ..}, true) => {
            player_move_or_attack(0, -1, game, objects);
            TookTurn
        },
        (Key {code: Down, ..}, true) | (Key {code: NumPad2, ..}, true) => {
            player_move_or_attack(0, 1, game, objects);
            TookTurn
        },
        (Key {code: Left, ..}, true) | (Key {code: NumPad4, ..}, true) => {
            player_move_or_attack(-1, 0, game, objects);
            TookTurn
        },
        (Key {code: Right, ..}, true) | (Key {code: NumPad6, ..}, true) => {
            player_move_or_attack(1, 0, game, objects);
            TookTurn
        },
        (Key {code: Home, ..}, true) | (Key {code: NumPad7, ..}, true) => {
            player_move_or_attack(-1, -1, game, objects);
            TookTurn
        },
        (Key {code: PageUp, ..}, true) | (Key {code: NumPad9, ..}, true) => {
            player_move_or_attack(1, -1, game, objects);
            TookTurn
        },
        (Key {code: End, ..}, true) | (Key {code: NumPad1, ..}, true) => {
            player_move_or_attack(-1, 1, game, objects);
            TookTurn
        },
        (Key {code: PageDown, ..}, true) | (Key {code: NumPad3, ..}, true) => {
            player_move_or_attack(1, 1, game, objects);
            TookTurn
        },
        (Key {code: NumPad5, ..}, true) => {
            TookTurn // do nothing, i.e. wait for the monster to come to you
        },
        (Key {printable: 'g', ..}, true) => {
            // pick up an item
            let item_id = objects.iter().position(|object| {
                object.pos() == objects[PLAYER].pos() && object.item.is_some()
            });
            if let Some(item_id) = item_id {
                pick_item_up(item_id, objects, game);
            }
            DidntTakeTurn
        },
        (Key {printable: 'i', ..}, true) => {
            // show the inventory: if an item is selcted, use it
            let inventory_index = inventory_menu(game,
                                                 "Press the key next to an item to use it, or any other to cancel. \n",
                                                  &mut tcod.root);
            if let Some(inventory_index) = inventory_index {
                use_item(inventory_index, objects, game, tcod)
            }
            DidntTakeTurn
        },
        (Key {printable: 'd', ..}, true) => {
            // show the inventory; if an item is selcted, drop it
            let inventory_index = inventory_menu(game
                , "Press the key next to an item to drop it, or any other to cancel. \n"
                , &mut tcod.root);
            if let Some(inventory_index) = inventory_index {
                drop_item(inventory_index, game, objects);
            }
            DidntTakeTurn
        },
        (Key {printable: ',' ,shift: true, ..}, true) => {
            // go down stairs, if player is on them
            let player_on_stairs = objects.iter().any(|object| {
                object.pos() == objects[PLAYER].pos() && object.name == "stairs"
            });
            if player_on_stairs {
                next_level(tcod, objects, game);
            }
            DidntTakeTurn
        },
        (Key {printable: 'c', ..}, true) => {
            // show character information
            let player = &objects[PLAYER];
            let level = player.level;
            let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
            if let Some(fighter) = player.fighter.as_ref() {
                let msg = format!("Character information

Level: {}
Experience: {}
Experience to level up: {}

Maximum HP: {}
Attack: {}
Defense: {}", level, fighter.xp, level_up_xp, player.max_hp(game), player.power(game), player.defense(game));
                msgbox(&msg, CHARACTER_SCREEN_WIDTH, &mut tcod.root);
            }
            DidntTakeTurn
        },
        _ => DidntTakeTurn,
    }
}

fn new_game (tcod: &mut Tcod) -> (Vec<Object>, Game) {
    // create object representing the player
    let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
    player.alive = true;
    player.fov_radius = 5; // start here, see how it goes
    player.fighter = Some(Fighter{base_max_hp: 100, hp: 100, base_defense: 1, base_power: 2, on_death: DeathCallback::Player, xp: 0});

    // the list of objects with just the player
    let mut objects = vec![player];
    let level = 1;
    
    let mut game = Game {
        // generate map (at thsi point it's not drawn to the screen)
        map: make_map(&mut objects, level),
        // create the list of game messages and their colors, starts empty
        log: vec![],
        inventory: vec![],
        dungeon_level: level,
    };

    // initial equipment: a dagger
    let mut dagger = Object::new(0, 0, '-', "dagger", colors::SKY, false);
    dagger.item = Some(Item::Sword);
    dagger.equipment = Some(Equipment {
        equipped: true,
        slot: Slot::LeftHand,
        max_hp_bonus: 0,
        defense_bonus: 0,
        power_bonus: 2
    });
    game.inventory.push(dagger);

    initialize_fov(&game.map, tcod);

    // a warm welcoming message!
    game.log.add("Welcome stranger! Prepare to perish in the Tombs of the Ancient Kings.", colors::RED);

    (objects, game)
}

fn play_game(objects: &mut Vec<Object>, game: &mut Game, tcod: &mut Tcod) {
    // force FOV 'recompute' first time through the game loop
    let mut previous_player_position = (-1, -1);

    let mut key = Default::default();

    while !tcod.root.window_closed() {
        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => key = k,
            _ => key = Default::default(),
        }

        // update player fov_radius if necessary
        if game.map[objects[PLAYER].x as usize][objects[PLAYER].y as usize].lit {
            objects[PLAYER].fov_radius = TORCH_RADIUS_IN_LIT_AREA;
        } else { // player is in dark area
            objects[PLAYER].fov_radius += 1;
            if objects[PLAYER].fov_radius > TORCH_RADIUS_IN_DARK_AREA {
                objects[PLAYER].fov_radius = TORCH_RADIUS_IN_DARK_AREA;
            }
        }

        // render the screen
        let fov_recompute = previous_player_position != (objects[PLAYER].pos()); // we may need to update this to account for changing fovs
        render_all(tcod, objects, game, fov_recompute); 

        tcod.root.flush();

        // level up if needed
        level_up(objects, game, tcod);

        // erase all objects at their old locations, before they move
        for object in objects.iter_mut() {
            object.clear(&mut tcod.con)
        }

        // handle keys and exit game if needed
        previous_player_position = objects[PLAYER].pos();
        let player_action = handle_keys(key, tcod, game, objects);
        if player_action == PlayerAction::Exit {
            save_game(objects, game).unwrap();
            break
        }

        // let monsters take their turn
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, game, objects, &tcod.fov);
                }
            }
        }
    }
}

fn main_menu(tcod: &mut Tcod) {
    let img = tcod::image::Image::from_file("menu_background.png")
        .ok().expect("Background image not found");
    
    while !tcod.root.window_closed() {
        // show the background image, at twice the regular console resolution
        tcod::image::blit_2x(&img, (0, 0), (-1, -1), &mut tcod.root, (0, 0));

        // add the title and some credits
        tcod.root.set_default_foreground(colors::LIGHT_YELLOW);
        tcod.root.print_ex(SCREEN_WIDTH / 2, SCREEN_HEIGHT / 2 - 4,
            BackgroundFlag::None, TextAlignment::Center, "TOMBS OF THE ANCIENT KINGS");
        tcod.root.print_ex(SCREEN_WIDTH / 2, SCREEN_HEIGHT - 2,
            BackgroundFlag::None, TextAlignment::Center, "By Nuzcraft");

        // show the options and wait for the player's choice
        let choices = &["Play a new game", "Continue last game", "Quit"];
        let choice = menu("", choices, 24, &mut tcod.root);

        match choice {
            Some(0) => {
                // new game
                let (mut objects, mut game) = new_game(tcod);
                play_game(&mut objects, &mut game, tcod);
            }
            Some(1) => {
                // load game
                match load_game() {
                    Ok((mut objects, mut game)) => {
                        initialize_fov(&game.map, tcod);
                        play_game(&mut objects, &mut game, tcod);
                    }
                    Err(_e) => {
                        msgbox("\nNo saved game to load. \n.", 24, &mut tcod.root);
                        continue;
                    }
                }
            }
            Some(2) => {
                // quit
                break;
            }
            _ => {}
        }
    }
}

fn save_game(objects: &[Object], game: &Game) -> Result<(), Box<Error>> {
    let save_data = serde_json::to_string(&(objects, game))?;
    let mut file = File::create("savegame")?;
    file.write_all(save_data.as_bytes())?;
    Ok(())
}

fn load_game() -> Result<(Vec<Object>, Game), Box<Error>> {
    let mut json_save_state = String::new();
    let mut file = File::open("savegame")?;
    file.read_to_string(&mut json_save_state)?;
    let result = serde_json::from_str::<(Vec<Object>, Game)>(&json_save_state)?;
    Ok(result)
}

/// advance to the next level
fn next_level(tcod: &mut Tcod, objects: &mut Vec<Object>, game: &mut Game) {
    game.log.add("You take a moment to rest and recover your strength.", colors::VIOLET);
    let heal_hp = objects[PLAYER].max_hp(game) / 2;
    objects[PLAYER].heal(heal_hp, game);

    game.log.add("After a rare moment of peace, you descend deepter into \
        the heart of the dungeon...", colors::RED);
    game.dungeon_level += 1;
    game.map = make_map(objects, game.dungeon_level);
    initialize_fov(&game.map, tcod);
}
