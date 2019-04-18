/// this file will be used to render everything to the screen
use crate::constants::*;
use crate::user_defined::*;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap}; // the 'Map as FovMap' section renames the tcod fov map
                                // so that it doesn't conflict with our user defined Map
use tcod::input::{Mouse};

/// this function will handle all the rendering needed
pub fn render_all(tcod: &mut Tcod, objects: &[Object], game: &mut Game, fov_recompute: bool) {
    if fov_recompute {
        // recompute FOV if needed (the player moved or something)
        let player = &objects[PLAYER];
        tcod.fov.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);

        // draw the map tiles, setting background colors
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let visible = tcod.fov.is_in_fov(x, y);
                let wall = game.map[x as usize][y as usize].block_sight;
                let color = match(visible, wall) {
                    // outside field of view
                    (false, true) => COLOR_DARK_WALL,
                    (false, false) => COLOR_DARK_GROUND,
                    // inside fov:COLOR_DARK_GROUND
                    (true, true) => COLOR_LIGHT_WALL,
                    (true, false) => COLOR_LIGHT_GROUND,    
                };
                let explored = &mut game.map[x as usize][y as usize].explored;
                if visible {
                    // since it's visible, explore it
                    *explored = true;
                }
                if *explored {
                    tcod.con.set_char_background(x, y, color, BackgroundFlag::Set);
                }
            }
        }
    }
    let mut to_draw: Vec<_> = objects.iter().filter(|o| {
        tcod.fov.is_in_fov(o.x, o.y) || 
        (o.always_visible && game.map[o.x as usize][o.y as usize].explored)
    }).collect();
    // sort so that non-blocking objects come first
    to_draw.sort_by(|o1, o2| {o1.blocks.cmp(&o2.blocks)});
    // draw all objects in the list
    for object in &to_draw {
        object.draw(&mut tcod.con);
    }

    // prepare to render the GUI panel
    tcod.panel.set_default_background(colors::BLACK);
    tcod.panel.clear();

    // show the player's stats
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].max_hp(game);
    render_bar(&mut tcod.panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, colors::LIGHT_RED, colors::DARKER_RED);

    // show the level of the dungeon
    tcod.panel.print_ex(1, 3, BackgroundFlag::None, TextAlignment::Left,
        format!("Dungeon level: {}", game.dungeon_level));

    // print the game messages, one line at a time
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in game.log.iter().rev() {
        let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        tcod.panel.set_default_foreground(color);
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
