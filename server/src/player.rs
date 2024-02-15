use crate::{BREAKABLE, EMPTY, MAZE_HEIGHT, MAZE_WIDTH, PLAYER, TILE_SIZE, WALL};
use macroquad::prelude as mq;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Position {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

impl Position {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self { x: 1.0, y: 0.0 }
    }
}

type Direction = Position;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Player {
    pub(crate) id: usize,
    pub(crate) pos: Position,
    direction: Direction,
    angle: f32,          // in radians
    angle_vertical: f32, // in radians
    pub(crate) action: String,
    pub(crate) name: String,
    pub(crate) score: u32,
}
impl Player {
    pub(crate) fn new(pos: Position, id: usize, name: String) -> Self {
        Self {
            id,
            pos,
            direction: Direction::default(),
            angle: 0.0,
            angle_vertical: 0.0,
            action: String::from(""),
            name,
            score: 0,
        }
    }
    pub fn touching_wall(&mut self, move_vec: mq::Vec2, maze: &mut [u8], moved: &mut bool) {
        let new_x = self.pos.x + TILE_SIZE * move_vec.x;
        let new_y = self.pos.y + TILE_SIZE * move_vec.y;

        let map_x = (new_x / TILE_SIZE) as usize;
        let map_y = (new_y / TILE_SIZE) as usize;
        let map_index = map_y * MAZE_WIDTH + map_x;

        if maze[map_index] == EMPTY {
            //set the current positions tile to 0
            let current_map_x = (self.pos.x / TILE_SIZE) as usize;
            let current_map_y = (self.pos.y / TILE_SIZE) as usize;
            let current_map_index = current_map_y * MAZE_WIDTH + current_map_x;
            maze[current_map_index] = 0;
            self.pos.x = new_x;
            self.pos.y = new_y;

            maze[map_index] = PLAYER;
            self.action = String::from("");
            *moved = true;
        }
    }

    pub fn input(&mut self, maze: &mut Vec<u8>, moved: &mut bool) -> Option<u32> {
        if self.action == "left" {
            self.angle -= std::f32::consts::FRAC_PI_2;
            self.action = String::from("");
            *moved = true;
        }
        if self.action == "right" {
            self.angle += std::f32::consts::FRAC_PI_2;
            self.action = String::from("");
            *moved = true;
        }

        if self.action == "shoot" {
            // Convert player position to grid coordinates
            let grid_x = (self.pos.x / TILE_SIZE).floor() as usize;
            let grid_y = (self.pos.y / TILE_SIZE).floor() as usize;

            // Determine direction to step through the map based on angle
            let step_x = self.angle.cos().round() as isize; // Round to ensure we move strictly in grid directions
            let step_y = self.angle.sin().round() as isize;

            // Initialize variables for iteration
            let mut current_x = grid_x as isize + step_x;
            let mut current_y = grid_y as isize + step_y;
            const MAX_RANGE: i32 = 6;
            let mut distance = 0;

            while current_x >= 0
                && current_x < MAZE_WIDTH as isize
                && current_y >= 0
                && current_y < MAZE_HEIGHT as isize
            {
                if distance > MAX_RANGE {
                    break;
                }

                let idx = (current_y * MAZE_WIDTH as isize + current_x) as usize;
                // if idx is 2 return none and break
                if maze[idx] == WALL {
                    break;
                }

                if maze[idx] == PLAYER || maze[idx] == BREAKABLE {
                    if maze[idx] == PLAYER {
                        self.score += 1;
                    }
                    maze[idx] = EMPTY;
                    *moved = true;
                    self.action = String::from("");
                    return Some(idx as u32);
                }
                // Move to the next tile in the direction
                current_x += step_x;
                current_y += step_y;
                distance += 1;
            }

            self.action = String::from(""); // Clear action after processing
        }

        self.direction = Direction::new(self.angle.cos(), self.angle.sin());

        let mut move_vec = mq::Vec2::new(0.0, 0.0);
        // Updated so you move one tile at a time

        if self.action == "W" {
            move_vec = mq::Vec2::new(self.direction.x, self.direction.y);
        }
        if self.action == "S" {
            move_vec = mq::Vec2::new(-self.direction.x, -self.direction.y);
        }
        if self.action == "D" {
            move_vec = mq::Vec2::new(-self.direction.y, self.direction.x);
        }
        if self.action == "A" {
            move_vec = mq::Vec2::new(self.direction.y, -self.direction.x);
        }

        if move_vec.length() > 0.0 {
            self.touching_wall(move_vec, maze, moved);
        }

        if self.pos.x < 0.0 {
            self.pos.x = 0.0;
        } else if self.pos.x > MAZE_WIDTH as f32 * TILE_SIZE {
            self.pos.x = MAZE_WIDTH as f32 * TILE_SIZE;
        }

        if self.pos.y < 0.0 {
            self.pos.y = 0.0;
        } else if self.pos.y > MAZE_HEIGHT as f32 * TILE_SIZE {
            self.pos.y = MAZE_HEIGHT as f32 * TILE_SIZE;
        }
        None
    }
}
