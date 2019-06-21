/// this file will be used to render everything to the screen
use crate::constants::*;
use crate::user_defined::*;
use crate::helper;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap}; // the 'Map as FovMap' section renames the tcod fov map
                                // so that it doesn't conflict with our user defined Map
use tcod::input::{self, Event, Mouse};

/// this function will handle all the rendering needed
pub fn render_all(tcod: &mut Tcod, objects: &[Object], game: &mut Game, fov_recompute: bool) {

    let player = &objects[PLAYER];
    let player_lit = game.map[player.x as usize][player.y as usize].lit;

    if fov_recompute {

        // find objects that emit light, and set their fovs as well
        // we can't store them on the objects because we can't write the FOV to file when we save (and don't really want to)
        let mut emitter_fovs = vec![];
        for object in objects {
            if object.emitter.is_some() {
                // since it emits light, create an FOV
                let mut fov_map = helper::create_fov_map(game);
                fov_map.compute_fov(object.x, object.y, object.emitter.as_ref().map_or(0, |f| f.radius), FOV_LIGHT_WALLS, FOV_ALGO);
                emitter_fovs.push(fov_map);
            }
        }

        // we need to find out which tiles are lit so we can tell if the player is standing in the light
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                // also visible if in the light of an emitter
                let mut in_emitter_light: bool = false;
                for fov in &emitter_fovs {
                    in_emitter_light = fov.is_in_fov(x, y);
                    if in_emitter_light == true {
                        break;
                    }
                }
                // if the tile is in the emmitter light, set it to lit, else set lit to false. This should let us
                // light and unlight tiles, but allow previously lit tiles to be explored
                let lit = &mut game.map[x as usize][y as usize].lit;
                if in_emitter_light {
                    *lit = true;
                } else {
                    *lit = false;
                }
            }
        }

        // recompute the player's FOV. if standing on a lit tile, use TORCH_RADIUS_IN_LIT_AREA
        tcod.fov.compute_fov(player.x, player.y, player.fov_radius, FOV_LIGHT_WALLS, FOV_ALGO);

        // draw the map tiles, setting background colors
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let visible_to_player = tcod.fov.is_in_fov(x, y); // this is the players fov
                let wall = game.map[x as usize][y as usize].block_sight;
                let lit_tile = game.map[x as usize][y as usize].lit;

                // for now, make the tiles visible to the player or in emitter light the same color
                // add a match thing for whether the player is lit, so we can move to greyscale
                let mut color = match(visible_to_player || lit_tile, wall, player_lit) {
                    // outside field of view
                    (false, true, true) => COLOR_DARK_WALL,
                    (false, true, false) => colors::DARKEST_GREY, //greyscale
                    (false, false, true) => COLOR_DARK_GROUND,
                    (false, false, false) => colors::DARKER_GREY, //greyscale
                    // inside fov:COLOR_DARK_GROUND
                    (true, true, true) => COLOR_LIGHT_WALL,
                    (true, true, false) => colors::DARK_GREY, //greyscale
                    (true, false, true) => COLOR_LIGHT_GROUND, 
                    (true, false, false) => colors::GREY, //greyscale 
                };

                // if lit by torch, adjust the color a smidge
                if lit_tile {
                    if player_lit {
                        color = colors::lerp(color, colors::ORANGE, 0.5)
                    } else {
                        color = colors::lerp(color, colors::LIGHTER_GREY, 0.5)
                    }
                }

                let explored = &mut game.map[x as usize][y as usize].explored;
                if visible_to_player || lit_tile {
                    // since it's visible, explore it
                    *explored = true;
                }
                if *explored {
                    tcod.con.set_char_background(x, y, color, BackgroundFlag::Set);
                }
            }
        }
    }

    // draw objects that are a) in players fov b) in a lit area c) are always visible and in an explored area
    let mut to_draw: Vec<_> = objects.iter().filter(|o| {
        tcod.fov.is_in_fov(o.x, o.y) || 
        game.map[o.x as usize][o.y as usize].lit ||
        (o.always_visible && game.map[o.x as usize][o.y as usize].explored)
    }).collect();

    // sort so that non-blocking objects come first
    to_draw.sort_by(|o1, o2| {o1.blocks.cmp(&o2.blocks)});
    // draw all objects in the list
    // if player is standing in a lit tile use color, else use black
    if player_lit {
        for object in &to_draw {
            object.draw(&mut tcod.con);
        }
    } else {
        for object in &to_draw {
            object.draw_black(&mut tcod.con);
        }
    }

    // prepare to render the GUI panel
    tcod.panel.set_default_background(colors::BLACK);
    tcod.panel.clear();

    // show the player's stats
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].max_hp(game);
    // if player is standing in a lit tile, use red, else grey
    if player_lit {
        render_bar(&mut tcod.panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, colors::LIGHT_RED, colors::DARKER_RED);
    } else {
        render_bar(&mut tcod.panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, colors::DARKER_GREY, colors::DARKEST_GREY);
    }

    // show the level of the dungeon
    tcod.panel.print_ex(1, 3, BackgroundFlag::None, TextAlignment::Left,
        format!("Dungeon level: {}", game.dungeon_level));

    // show whether the player is in a lit or dark tile
    tcod.panel.print_ex(1, 5, BackgroundFlag::None, TextAlignment::Left,
        match player_lit {
            true => "Lit",
            false => "Dark",
        });

    // print the game messages, one line at a time
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in game.log.iter().rev() {
        let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        // if player is standing in a lit tile, use color, else just white
        if player_lit {
            tcod.panel.set_default_foreground(color);
        } else {
            tcod.panel.set_default_foreground(colors::WHITE);
        }
        tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }

    // display names of objects under the mouse
    tcod.panel.set_default_foreground(colors::LIGHT_GREY);
    tcod.panel.print_ex(1, 0, BackgroundFlag::None, TextAlignment::Left, 
                   get_names_under_mouse(tcod.mouse, objects, &mut tcod.fov));

    // blit the contents of the 'panel' to the root console
    blit(&tcod.panel, (0, 0), (SCREEN_WIDTH, PANEL_HEIGHT), &mut tcod.root, (0, PANEL_Y), 1.0, 1.0);

    // blit the con to the root
    blit(&tcod.con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), &mut tcod.root, (0, 0), 1.0, 1.0); 

}

fn render_bar(panel: &mut Offscreen,
              x: i32,
              y: i32,
              total_width: i32,
              name: &str,
              value: i32,
              maximum: i32,
              bar_color: Color,
              back_color: Color,) {
    // render a bar (HP, exp, etc). First calculate the width of the bar
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;

    // render the background first
    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

    // now, render the bar on top
    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }

    // finally, some centered text with the values
    panel.set_default_foreground(colors::WHITE);
    panel.print_ex(x + total_width / 2, y, BackgroundFlag::None, TextAlignment::Center,
                   &format!("{}: {}/{}", name, value, maximum));
}

fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    // create a list with the names of all objects at the mouse's coordinates and in fov
    let names = objects
        .iter()
        .filter(|obj| {obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y)})
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ") // join the names, separated by commas
}

pub fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
    // cannot have more than 26 options (a-z)
    assert!(options.len() <= 26, "Cannot have a menu with more than 26 options.");

    // calculate total height for the header (after auto-wrap) and one line per option
    let header_height = if header.is_empty() {
        0
    } else {
        root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header)
    };
    let height = options.len() as i32 + header_height;

    // create an offscreen console that represents the menu's window
    let mut window = Offscreen::new(width, height);

    // print the header, with auto-wrap
    window.set_default_foreground(colors::WHITE);
    window.print_rect_ex(0, 0, width, height, BackgroundFlag::None, TextAlignment::Left, header);

    // print all the options
    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        window.print_ex(0, header_height + index as i32, BackgroundFlag::None, TextAlignment::Left, text);
    }

    // blit the contents of 'window' to the root console
    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    tcod::console::blit(&mut window, (0, 0), (width, height), root, (x, y), 1.0, 0.7);

    // present the root console tot he player and wait for keypress
    root.flush();
    let key = root.wait_for_keypress(true);

    // convert the ASCII code to an index; if it correspons to an option, return it
    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn msgbox(text: &str, width: i32, root: &mut Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}

/// return the position of a tile left-clicked in player's FOV (optionally in a 
/// range), or (None, None) if right clicked.
pub fn target_tile(tcod: &mut Tcod,
                objects: &[Object],
                game: &mut Game,
                max_range: Option<f32>) -> Option<(i32, i32)> {
    use tcod::input::KeyCode::Escape;
    loop {
        // render the screen. This erases the inventory and shows the names of
        // objects under the mouse.
        tcod.root.flush();
        let event = input::check_for_event(input::KEY_PRESS | input::MOUSE).map(|e| e.1);
        let mut key = None;
        match event {
            Some(Event::Mouse(m)) => tcod.mouse = m,
            Some(Event::Key(k)) => key = Some(k),
            None => {}
        }
        render_all(tcod, objects, game, false);
        let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);

        // accept the target if the player clicked in FOV, and in case a range
        // is specified, if  it's within that range
        let in_fov = (x < MAP_WIDTH) && (y < MAP_HEIGHT) && tcod.fov.is_in_fov(x, y);
        let in_range = max_range.map_or(true, |range| objects[PLAYER].distance(x, y) <= range);
        if tcod.mouse.lbutton_pressed && in_fov && in_range {
            return Some((x, y))
        }

        let escape = key.map_or(false, |k| k.code == Escape);
        if tcod.mouse.rbutton_pressed || escape {
            return None // cancel if the player right-clicked or pressed Escape
        }
    }
}

pub fn target_monster(tcod: &mut Tcod,
                objects: &[Object],
                game: &mut Game,
                max_range: Option<f32>) -> Option<usize> {
    loop {
        match target_tile(tcod, objects, game, max_range) {
            Some((x, y)) => {
                // return the first clicked monster, otherwise continue looping
                for (id, obj) in objects.iter().enumerate() {
                    if obj.pos() == (x, y) && obj.fighter.is_some()  && id != PLAYER {
                        return Some(id)
                    }
                }
            }
            None => return None,
        }
    }
}

pub fn inventory_menu(game: &mut Game, header: &str, root: &mut Root) -> Option<usize> {
    // show a menu with each item of the inventory as an option
    let options = if game.inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
    } else {
        game.inventory.iter().map(|item| {
            // show additional information, in case it's equipped
            match item.equipment {
                Some(equipment) if equipment.equipped => {
                    format!("{} (on {})", item.name, equipment.slot)
                }
                _ => item.name.clone()
            }
        }).collect()
    };

    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);

    // if an item was chosen, return it
    if game.inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

pub fn initialize_fov(map: &Map, tcod: &mut Tcod) {
    // create the FOV map, according to the generated map
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(x, y,
                !map[x as usize][y as usize].block_sight,
                !map[x as usize][y as usize].blocked);
        }
    }
    tcod.con.clear() // unexplored areas start black (which is the default background color)
}
