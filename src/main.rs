/// Rust/libtcod tutorial, with notes

// tcod is an external crate (and is referenced in the Cargo.toml file)
extern crate tcod;
extern crate rand;

use std::cmp;
use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap, FovAlgorithm}; // the 'Map as FovMap' section renames the tcod fov map
                                              // so that it doesn't conflict with our user defined Map
use rand::Rng;

// const are constants that cannot be changed in code
// actual size of the screen
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const LIMIT_FPS: i32 = 20; // limit frames per second

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

// parameters for dungeon generator
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;
const MAX_ROOM_MONSTERS: i32 = 3;

const COLOR_DARK_WALL: Color = Color{r: 0, g: 0, b: 100};
const COLOR_LIGHT_WALL: Color = Color{r: 130, g: 110, b: 50};
const COLOR_DARK_GROUND: Color = Color{r: 50, g: 50, b: 150};
const COLOR_LIGHT_GROUND: Color = Color{r: 200, g: 180, b: 50};

//fov
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true; // light walls or not
const TORCH_RADIUS: i32 = 10;

// player will always be the first object
const PLAYER: usize = 0;

type Map = Vec<Vec<Tile>>; // a MAP is 2 dimensional vector of tiles

// this is a generic object. Anything represented by a character on the screen
// player, monster, stairs, item, etc
#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, color: Color) -> Self {
        Object {
            x: x,
            y: y,
            char: char,
            color: color,
        }
    }

    pub fn move_by(&mut self, dx: i32, dy: i32, map: &Map) {
        // move by the given amount
        if !map[(self.x + dx) as usize][(self.y + dy) as usize].blocked {
            self.set_pos(self.x + dx, self.y + dy);
        }
    }

    /// set the color, then draw the character that represents this object at its position
    pub fn draw(&self, con: &mut Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }

    /// Erase the character that represents this object
    pub fn clear(&self, con: &mut Console) {
        con.put_char(self.x, self.y, ' ', BackgroundFlag::None);
    }

    // returns the current position
    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    // sets a new position for an object
    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
}

// a tile of the map and its properties
#[derive(Clone, Copy, Debug)]
struct Tile {
    blocked: bool,
    block_sight: bool,
    explored: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile{blocked: false, block_sight: false, explored: false}
    }

    pub fn wall() -> Self {
        Tile{blocked: true, block_sight: true, explored: false}
    }
}

// a simple rectangle on the map, used to define a room
#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    pub fn new (x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect{x1: x, y1: y, x2: x + w, y2: y + h}
    }

    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    pub fn intersects_with(&self, other: &Rect) -> bool {
        // return true if this rectangle intersects with another one
        (self.x1 <= other.x2) && (self.x2 >= other.x1) &&
            (self.y1 <= other.y2) && (self.y2 >= other.y1)
    }
}

/// main function of the game, starts with initializers, then moves into the main game loop
fn main() {
    
    let mut root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod) // set up a font. this can be in various formats, must be in the root, next to Cargo.toml
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT) // set the dimensions of the window
        .title("Rust/libtcod tutorial") // name the window
        .init(); // this actually opens the window

    let mut con = Offscreen::new(MAP_WIDTH, MAP_HEIGHT); // create an offscreen console the same width and height as the root
    // we'll blit this to the root screen when we're ready    

    tcod::system::set_fps(LIMIT_FPS); // set the frames per second; limits the refresh rate

    // player variables
    let player = Object::new(0, 0, '@', colors::WHITE);
    let mut objects = vec![player];

    // map
    let mut map = make_map(&mut objects);
    // fov map
    // this creates an fovmap with the same dimensions as the entire map. it includes every
    // tile's position, and whether its transparent and walkable
    let mut fov_map = FovMap::new(MAP_WIDTH, MAP_HEIGHT);
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            fov_map.set(x, y, 
                        !map[x as usize][y as usize].block_sight,
                        !map[x as usize][y as usize].blocked);
        }
    }

    let mut previous_player_position = (-1, -1);    

    // main game loop
    while !root.window_closed() {
        con.set_default_foreground(colors::WHITE); // this is the color everything will be drawn in unless otherwise specified
        root.clear(); // clear the screen
        let fov_recompute = previous_player_position != (objects[PLAYER].x, objects[PLAYER].y); // only recompute fov if the player moved
        render_all(&mut root, &mut con, &objects, &mut map, &mut fov_map, fov_recompute); // render everything
        root.flush(); // draw everything to the window
        for object in &objects {
            object.clear(&mut con);
        }
        // handle keys and exit game if needed
        let player = &mut objects[PLAYER];
        previous_player_position = player.pos();
        let exit = handle_keys(&mut root, player, &map);
        if exit {
            break
        }
    }

}

/// this function will handle all interactions from the player
/// this will return false if the player wants to continue playing, true to quit
fn handle_keys(root: &mut Root, player: &mut Object, map: &Map) -> bool {

    use tcod::input::Key;
    use tcod::input::KeyCode::*;

    let key = root.wait_for_keypress(true);
    match key {
        Key {code: Enter, alt: true, ..} => {
            // Alt+Enter: toggel fullscreen
            let fullscreen = root.is_fullscreen();
            root.set_fullscreen(!fullscreen);
        },
        Key {code: Escape, ..} => return true,
        // movement keys
        Key {code: Up, ..} => player.move_by(0, -1, map),
        Key {code: Down, ..} => player.move_by(0, 1, map),
        Key {code: Left, ..} => player.move_by(-1, 0, map),
        Key {code: Right, ..} => player.move_by(1, 0, map),
        _ => {},
    }
    false
}

fn make_map(objects: &mut Vec<Object>) -> Map {
    // fill map with "unblocked" tiles
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

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
            place_objects(new_room, objects);

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

    // return the map and starting position
    map
}

/// this function will handle all the rendering needed
fn render_all(root: &mut Root, con: &mut Offscreen, objects: &[Object], map: &mut Map, fov_map: &mut FovMap, fov_recompute: bool) {
    if fov_recompute {
        // recompute FOV if needed (the player moved or something)
        let player = &objects[PLAYER];
        fov_map.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);

        // draw the map tiles, setting background colors
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let visible = fov_map.is_in_fov(x, y);
                let wall = map[x as usize][y as usize].block_sight;
                let color = match(visible, wall) {
                    // outside field of view
                    (false, true) => COLOR_DARK_WALL,
                    (false, false) => COLOR_DARK_GROUND,
                    // inside fov:COLOR_DARK_GROUND
                    (true, true) => COLOR_LIGHT_WALL,
                    (true, false) => COLOR_LIGHT_GROUND,    
                };
                let explored = &mut map[x as usize][y as usize].explored;
                if visible {
                    // since it's visible, explore it
                    *explored = true;
                }
                if *explored {
                    con.set_char_background(x, y, color, BackgroundFlag::Set);
                }
            }
        }
    }
    // draw all objects in the list
    for object in objects {
        if fov_map.is_in_fov(object.x, object.y) {
            object.draw(con);
        }
    }

    blit(con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), root, (0, 0), 1.0, 1.0); // blit the con to the root

}

fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
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

/// take a room and add objects to it (monsters, items, etc)
fn place_objects(room: Rect, objects: &mut Vec<Object>) {
    // choose a random number of monsters
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        // choose random spot for this monster
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        let monster = if rand::random::<f32>() < 0.8 { // 80% chance of getting an orc
            Object::new(x, y, 'o', colors::DESATURATED_GREEN)
        } else {
            Object::new(x, y, 'T', colors::DARKER_GREEN) // else, a troll
        };
        objects.push(monster);
    }
}
