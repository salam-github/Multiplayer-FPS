use macroquad::{miniquad::gl::GL_SMOOTH_POINT_SIZE_GRANULARITY, prelude as mq};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::UdpSocket;

const WINDOW_WIDTH: u32 = 1024;
const WINDOW_HEIGHT: u32 = 512;

const MAP_WIDTH: u32 = 8;
const MAP_HEIGHT: u32 = 8;
const TILE_SIZE: u32 = 64;

const NUM_RAYS: u32 = 512;
const RAYS_PER_SECOND: f32 = NUM_RAYS as f32 / 2.0;

const FOV: f32 = std::f32::consts::PI / 2.0;

const VIEW_DISTANCE: f32 = 7.0 * TILE_SIZE as f32;

const NUM_TEXTURES: i32 = 3;

fn window_conf() -> mq::Conf {
    mq::Conf {
        window_title: "3D Raycaster".to_owned(),
        window_width: WINDOW_WIDTH as i32,
        window_height: WINDOW_HEIGHT as i32,
        window_resizable: true,
        ..Default::default()
    }
}

struct ScalingInfo {
    width: f32,
    height: f32,
    offset: mq::Vec2,
}
impl ScalingInfo {
    fn new() -> ScalingInfo {
        let w = mq::screen_width();
        let h = mq::screen_height();

        let width = w.min(h * 2.0);
        let height = h.min(w / 2.0);
        let offset = mq::Vec2::new((w - width) / 2.0, (h - height) / 2.0);

        ScalingInfo {
            width,
            height,
            offset,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct PlayerUpdate {
    id: String,
    action: String,
}
#[derive(Serialize, Deserialize)]
struct GameState {
    players: Vec<Player>,
    map: Vec<u8>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Player {
    id: u8,
    pos: (f32, f32),
    direction: (f32, f32),
    angle: f32,          // in radians
    angle_vertical: f32, // in radians
    rayhits: Vec<(Ray, Option<RayHit>)>,
    last_input_time: f64,
    action: String,
}
impl Player {
    fn new(pos: (f32, f32), id: u8) -> Self {
        Self {
            id,
            pos,
            direction: (1.0 as f32, 0.0 as f32),
            angle: 0.0,
            angle_vertical: 0.0,
            rayhits: Vec::new(),
            last_input_time: 0.0,
            action: String::from(""),
        }
    }
    fn touching_wall(&mut self, move_vec: mq::Vec2, map: &[u8]) {
        let new_x = self.pos.0 + TILE_SIZE as f32 * move_vec.x;
        let new_y = self.pos.1 + TILE_SIZE as f32 * move_vec.y;

        let map_x = (new_x / TILE_SIZE as f32) as usize;
        let map_y = (new_y / TILE_SIZE as f32) as usize;
        let map_index = map_y * MAP_WIDTH as usize + map_x;

        if map[map_index] == 0 {
            // Assuming 0 is an empty tile
            self.pos.0 = new_x;
            self.pos.1 = new_y;
            println!("pos: {:?}", self.pos);
        }
    }
    fn input(&mut self, map: &[u8]) {
        // Updated so you turn 90 degrees at a time
        if self.action == "left" {
            self.angle -= std::f32::consts::FRAC_PI_2;
        }
        if self.action == "right" {
            self.angle += std::f32::consts::FRAC_PI_2;
        }

        // if self.angle < 0.0 {
        //     self.angle += 2.0 * std::f32::consts::PI;
        // } else if self.angle > 2.0 * std::f32::consts::PI {
        //     self.angle -= 2.0 * std::f32::consts::PI;
        // }
        // if self.angle_vertical > std::f32::consts::PI / 2.1 {
        //     self.angle_vertical = std::f32::consts::PI / 2.1;
        // } else if self.angle_vertical < -std::f32::consts::PI / 2.1 {
        //     self.angle_vertical = -std::f32::consts::PI / 2.1;
        // }

        self.direction = (self.angle.cos(), self.angle.sin());

        let mut move_vec = mq::Vec2::new(0.0, 0.0);
        // Updated so you move one tile at a time

        if self.action == "W" {
            move_vec = mq::Vec2::new(self.direction.0, self.direction.1);
            println!("W");
        }
        if self.action == "S" {
            move_vec = mq::Vec2::new(-self.direction.0, -self.direction.1);
        }
        if self.action == "A" {
            move_vec = mq::Vec2::new(-self.direction.1, self.direction.0);
        }
        if self.action == "D" {
            move_vec = mq::Vec2::new(self.direction.1, -self.direction.0);
        }

        if move_vec.length() > 0.0 {
            self.touching_wall(move_vec, map);
        }

        if self.pos.0 < 0.0 {
            self.pos.0 = 0.0;
        } else if self.pos.0 > MAP_WIDTH as f32 * TILE_SIZE as f32 {
            self.pos.0 = MAP_WIDTH as f32 * TILE_SIZE as f32;
        }

        if self.pos.1 < 0.0 {
            self.pos.1 = 0.0;
        } else if self.pos.1 > MAP_HEIGHT as f32 * TILE_SIZE as f32 {
            self.pos.1 = MAP_HEIGHT as f32 * TILE_SIZE as f32;
        }
    }

    fn cast_rays(&self, mut map: &mut [u8], num_rays: u32) -> Vec<(Ray, Option<RayHit>)> {
        let rotation_matrix = mq::Mat2::from_angle(self.angle);
        let center_ray_index = num_rays / 2; // Assuming an odd number of rays
        (0..num_rays)
            .map(|i| {
                let unrotated_direction =
                    mq::Vec2::new(1.0, (i as f32 / num_rays as f32 - 0.5) * FOV);
                let direction = rotation_matrix * unrotated_direction;

                let ray = Ray::new((self.pos.0, self.pos.1), (direction.x, direction.y));

                // Pass shots_fired only if it's the center ray
                // otherwise you destroy eveything in the cone created by rotated rays
                let mut shots_fired = false;
                if self.action == "shoot" && i == center_ray_index {
                    println!("Shots fired");
                    shots_fired = true;
                }
                ray.cast_ray(&mut map, shots_fired)
            })
            .collect()
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct RayHit {
    pos: (f32, f32),
    world_distance: f32,
    x_move: bool,
    wall_coord: f32, // 0-1.0 as x
    wall_type: u8,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
struct Ray {
    pos: (f32, f32),
    angle: f32,
    direction: (f32, f32),
}
impl Ray {
    fn new(pos: (f32, f32), direction: (f32, f32)) -> Self {
        Self {
            pos,
            angle: direction.1.atan2(direction.0),
            direction,
        }
    }
    fn cast_ray(&self, map: &mut [u8], shots_fired: bool) -> (Ray, Option<RayHit>) {
        // DDA algorithm
        let x = self.pos.0 / TILE_SIZE as f32; // (0.0, 8.0)
        let y = self.pos.1 / TILE_SIZE as f32; // (0.0, 8.0)
        let ray_start = mq::Vec2::new(x, y);

        let ray_dir = mq::Vec2::new(self.direction.0, self.direction.1).normalize();

        let ray_unit_step_size = mq::Vec2::new(
            (1.0 + (ray_dir.y / ray_dir.x).powi(2)).sqrt(),
            (1.0 + (ray_dir.x / ray_dir.y).powi(2)).sqrt(),
        );
        let mut map_check = ray_start.floor();
        let mut ray_length_1d = mq::Vec2::ZERO;
        let mut step = mq::Vec2::ZERO;

        if ray_dir.x < 0.0 {
            step.x = -1.0;
            ray_length_1d.x = (x - map_check.x) * ray_unit_step_size.x;
        } else {
            step.x = 1.0;
            ray_length_1d.x = (map_check.x + 1.0 - x) * ray_unit_step_size.x;
        }

        if ray_dir.y < 0.0 {
            step.y = -1.0;
            ray_length_1d.y = (y - map_check.y) * ray_unit_step_size.y;
        } else {
            step.y = 1.0;
            ray_length_1d.y = (map_check.y + 1.0 - y) * ray_unit_step_size.y;
        }

        let max_distance = 100.0;
        let mut distance = 0.0;
        let mut x_move;
        while distance < max_distance {
            if ray_length_1d.x < ray_length_1d.y {
                map_check.x += step.x;
                distance = ray_length_1d.x;
                ray_length_1d.x += ray_unit_step_size.x;
                x_move = true;
            } else {
                map_check.y += step.y;
                distance = ray_length_1d.y;
                ray_length_1d.y += ray_unit_step_size.y;
                x_move = false;
            }

            if map_check.x >= 0.0
                && map_check.x < MAP_WIDTH as f32
                && map_check.y >= 0.0
                && map_check.y < MAP_HEIGHT as f32
            {
                let map_index = (map_check.y * MAP_WIDTH as f32 + map_check.x) as usize;
                let wall_type = map[map_index];
                if wall_type != 0 {
                    // 0 = no wall
                    //  if shots fired set wall to 0 effectively removing it
                    if shots_fired {
                        map[map_index] = 0;
                    }

                    let pos = mq::Vec2::new(self.pos.0, self.pos.1)
                        + (ray_dir * distance * TILE_SIZE as f32);

                    let map_pos = pos / TILE_SIZE as f32;
                    let wall_pos = map_pos - map_pos.floor();
                    let wall_coord = if x_move { wall_pos.y } else { wall_pos.x };

                    return (
                        *self,
                        Some(RayHit {
                            pos: (pos.x, pos.y),
                            world_distance: distance * TILE_SIZE as f32,
                            x_move,
                            wall_coord,
                            wall_type,
                        }),
                    );
                }
            }
        }

        (*self, None)
    }
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let socket = UdpSocket::bind(addr).await.unwrap();
    println!("Server running on {}", addr);

    let mut clients: HashMap<SocketAddr, Player> = HashMap::new();
    let mut player_ids: HashMap<SocketAddr, u8> = HashMap::new();
    let mut player_count: usize = 0;
    let mut buf = [0u8; 1024];
    let mut players: Vec<Player> = Vec::new();
    // let scaling_info = ScalingInfo::new();
    let mut num_rays = 0.0;
    let input_threshold = 0.1; // 0.1 seconds between inputs, adjust as needed
    let mut map = [
        1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 3, 2, 0, 0, 1, 3, 0,
        0, 3, 3, 0, 0, 0, 0, 0, 0, 2, 3, 0, 0, 3, 0, 2, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 3, 3, 3,
        2, 1, 2, 1,
    ];
    let mut gamestate = GameState {
        players: Vec::new(),
        map: map.to_vec(),
    };

    loop {
        let (len, client_addr) = socket.recv_from(&mut buf).await.unwrap();
        let msg = String::from_utf8_lossy(&buf[..len]);

        if msg == "new_connection" {
            if !clients.contains_key(&client_addr) {
                player_count += 1;
                let id = player_count.to_string();
                println!("New player connected: {}", id);
                socket.send_to(id.as_bytes(), client_addr).await.unwrap();
                let new_player = Player::new(
                    mq::Vec2::new(
                        WINDOW_WIDTH as f32 / 4.0 + TILE_SIZE as f32 / 2.0,
                        WINDOW_HEIGHT as f32 / 2.0 + TILE_SIZE as f32 / 2.0,
                    )
                    .into(),
                    player_count as u8,
                );
                clients.insert(client_addr, new_player.clone());
                // player_ids.insert(client_addr, new_player.id);
                //add player to players
                players.push(new_player.clone());
            }
        } else if let Ok(update) = serde_json::from_str::<PlayerUpdate>(&msg) {
            println!("Received action update from {}", client_addr);
            println!("update: {:?}", update.action);
            // Update players action in the vector of players
            for player in players.iter_mut() {
                if player.id == update.id.parse::<u8>().unwrap() {
                    player.action = update.action.clone();
                }
            }
        }

        //for each player in the players vector, send the updated game state to all clients
        for player in players.iter_mut() {
            let floor_level = (WINDOW_HEIGHT as f32 / 2.0)
                * (1.0 + player.angle_vertical.tan() / (FOV / 2.0).tan());
            //??????
            //    let delta = mq::get_frame_time(); // seconds

            //used for input throttling
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();

            // Check if enough time has elapsed since the last input
            if current_time - player.last_input_time >= input_threshold {
                player.input(&map);
                player.last_input_time = current_time; // Update the last input time
            }
            //????
            // if num_rays < NUM_RAYS as f32 {
            //     num_rays += delta * RAYS_PER_SECOND;
            // } else {
            //     num_rays = NUM_RAYS as f32;
            // }
            player.rayhits = player.cast_rays(&mut map, num_rays as u32);
        }

        //update the game state
        gamestate.players = players.clone();
        //broadcast the game state to all clients
        let broadcast_msg = serde_json::to_string(&gamestate).unwrap();
        for &addr in clients.keys() {
            println!("Sending update to {}", addr);
            println!("broadcast_msg: {:?}", broadcast_msg);
            socket
                .send_to(broadcast_msg.as_bytes(), addr)
                .await
                .unwrap();

            println!("stuck here");
        }
        println!("stuck here");
    }
    // #[rustfmt::skip]
}
