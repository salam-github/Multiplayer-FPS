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

#[derive(Serialize, Deserialize, Clone)]
struct PlayerUpdate {
    id: String,
    action: String,
}

struct Player {
    id: u8,
    pos: mq::Vec2,
    direction: mq::Vec2,
    angle: f32,          // in radians
    angle_vertical: f32, // in radians
}
impl Player {
    fn new(pos: mq::Vec2, id: u8) -> Self {
        Self {
            id,
            pos,
            angle: 0.0,
            angle_vertical: 0.0,
            direction: mq::Vec2::new(1.0, 0.0),
        }
    }
    fn touching_wall(&mut self, move_vec: mq::Vec2, map: &[u8]) {
        let new_x = self.pos.x + TILE_SIZE as f32 * move_vec.x;
        let new_y = self.pos.y + TILE_SIZE as f32 * move_vec.y;

        let map_x = (new_x / TILE_SIZE as f32) as usize;
        let map_y = (new_y / TILE_SIZE as f32) as usize;
        let map_index = map_y * MAP_WIDTH as usize + map_x;

        if map[map_index] == 0 {
            // Assuming 0 is an empty tile
            self.pos.x = new_x;
            self.pos.y = new_y;
            println!("pos: {:?}", self.pos);
        }
    }
    fn input(&mut self, delta: f32, map: &[u8]) {
        // Updated so you turn 90 degrees at a time
        if mq::is_key_down(mq::KeyCode::Left) {
            self.angle -= std::f32::consts::FRAC_PI_2;
        }
        if mq::is_key_down(mq::KeyCode::Right) {
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

        self.direction = mq::Vec2::new(self.angle.cos(), self.angle.sin());

        let mut move_vec = mq::Vec2::new(0.0, 0.0);
        // Updated so you move one tile at a time

        if mq::is_key_down(mq::KeyCode::W) {
            move_vec = mq::Vec2::new(self.direction.x, self.direction.y);
            println!("W");
        }
        if mq::is_key_down(mq::KeyCode::S) {
            move_vec = mq::Vec2::new(-self.direction.x, -self.direction.y);
        }
        if mq::is_key_down(mq::KeyCode::D) {
            move_vec = mq::Vec2::new(-self.direction.y, self.direction.x);
        }
        if mq::is_key_down(mq::KeyCode::A) {
            move_vec = mq::Vec2::new(self.direction.y, -self.direction.x);
        }

        if move_vec.length() > 0.0 {
            self.touching_wall(move_vec, map);
        }

        if self.pos.x < 0.0 {
            self.pos.x = 0.0;
        } else if self.pos.x > MAP_WIDTH as f32 * TILE_SIZE as f32 {
            self.pos.x = MAP_WIDTH as f32 * TILE_SIZE as f32;
        }

        if self.pos.y < 0.0 {
            self.pos.y = 0.0;
        } else if self.pos.y > MAP_HEIGHT as f32 * TILE_SIZE as f32 {
            self.pos.y = MAP_HEIGHT as f32 * TILE_SIZE as f32;
        }
    }

    fn cast_rays(
        &self,
        mut map: &mut [u8],
        num_rays: u32,
        shots_fired: bool,
    ) -> Vec<(Ray, Option<RayHit>)> {
        let rotation_matrix = mq::Mat2::from_angle(self.angle);
        let center_ray_index = num_rays / 2; // Assuming an odd number of rays
        (0..num_rays)
            .map(|i| {
                let unrotated_direction =
                    mq::Vec2::new(1.0, (i as f32 / num_rays as f32 - 0.5) * FOV);
                let direction = rotation_matrix * unrotated_direction;
                let ray = Ray::new(self.pos, direction);

                // Pass shots_fired only if it's the center ray
                // otherwise you destroy eveything in the cone created by rotated rays
                let is_shots_fired = shots_fired && i == center_ray_index;
                ray.cast_ray(&mut map, is_shots_fired)
            })
            .collect()
    }
}

struct RayHit {
    pos: mq::Vec2,
    world_distance: f32,
    x_move: bool,
    wall_coord: f32, // 0-1.0 as x
    wall_type: u8,
}
#[derive(Clone, Copy)]
struct Ray {
    pos: mq::Vec2,
    angle: f32,
    direction: mq::Vec2,
}
impl Ray {
    fn new(pos: mq::Vec2, direction: mq::Vec2) -> Self {
        Self {
            pos,
            angle: direction.y.atan2(direction.x),
            direction,
        }
    }
    fn cast_ray(&self, map: &mut [u8], shots_fired: bool) -> (Ray, Option<RayHit>) {
        // DDA algorithm
        let x = self.pos.x / TILE_SIZE as f32; // (0.0, 8.0)
        let y = self.pos.y / TILE_SIZE as f32; // (0.0, 8.0)
        let ray_start = mq::Vec2::new(x, y);

        let ray_dir = self.direction.normalize();

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

                    let pos = self.pos + (ray_dir * distance * TILE_SIZE as f32);

                    let map_pos = pos / TILE_SIZE as f32;
                    let wall_pos = map_pos - map_pos.floor();
                    let wall_coord = if x_move { wall_pos.y } else { wall_pos.x };

                    return (
                        *self,
                        Some(RayHit {
                            pos,
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

    loop {
        let (len, client_addr) = socket.recv_from(&mut buf).await.unwrap();
        let msg = String::from_utf8_lossy(&buf[..len]);

        if msg == "new_connection" {
            if !clients.contains_key(&client_addr) {
                player_count += 1;
                let id = format!("player{}", player_count);
                println!("New player connected: {}", id);
                socket.send_to(id.as_bytes(), client_addr).await.unwrap();
                let new_player = Player::new(
                    mq::Vec2::new(
                        WINDOW_WIDTH as f32 / 4.0 + TILE_SIZE as f32 / 2.0,
                        WINDOW_HEIGHT as f32 / 2.0 + TILE_SIZE as f32 / 2.0,
                    ),
                    player_count as u8,
                );
                clients.insert(client_addr, new_player);
                // player_ids.insert(client_addr, new_player.id);
            }
        } else if let Ok(update) = serde_json::from_str::<PlayerUpdate>(&msg) {
            println!("Received action update from {}", client_addr);
            println!("update: {:?}", update.action);

            // clients.insert(client_addr, update.action);

            // Broadcast updates to all clients except the sender
            // let all_positions: Vec<_> = clients
            //     .iter()
            //     .filter_map(|(&addr, pos)| {
            //         if addr != client_addr {
            //             Some(PlayerUpdate {
            //                 id: player_ids[&addr].clone(),
            //                 position: pos.clone(),
            //             })
            //         } else {
            //             None
            //         }
            //     })
            //     .collect();

            // let broadcast_msg = serde_json::to_string(&all_positions).unwrap();
            // for &addr in clients.keys() {
            //     if addr != client_addr {
            //         println!("Sending update to {}", addr);
            //         socket
            //             .send_to(broadcast_msg.as_bytes(), addr)
            //             .await
            //             .unwrap();
            //     }
            // }
        }
    }

    #[rustfmt::skip]
    let mut map = [
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 2,
        2, 0, 0, 0, 0, 0, 0, 3,
        2, 0, 0, 1, 3, 0, 0, 3,
        3, 0, 0, 0, 0, 0, 0, 2,
        3, 0, 0, 3, 0, 2, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 3, 3, 3, 2, 1, 2, 1,
    ];

    let mut num_rays = 0.0;

    //used for input throttling
    let mut last_input_time = 0.0; // Tracks the last time player.input() was called
    let input_threshold = 0.1; // 0.1 seconds between inputs, adjust as needed

    // loop {
    //     let scaling_info = ScalingInfo::new();
    //     let shots_fired = mq::is_key_pressed(mq::KeyCode::Space);
    //     if shots_fired {
    //         println!("shots fired");
    //     }

    //     let floor_level =
    //         (WINDOW_HEIGHT as f32 / 2.0) * (1.0 + player.angle_vertical.tan() / (FOV / 2.0).tan());

    //     let delta = mq::get_frame_time(); // seconds

    //     //used for input throttling
    //     let current_time = mq::get_time();

    //     // Check if enough time has elapsed since the last input
    //     if current_time - last_input_time >= input_threshold {
    //         player.input(delta, &map);
    //         last_input_time = current_time; // Update the last input time
    //     }

    //     if num_rays < NUM_RAYS as f32 {
    //         num_rays += delta * RAYS_PER_SECOND;
    //     } else {
    //         num_rays = NUM_RAYS as f32;
    //     }
    //     let ray_touches = player.cast_rays(&mut map, num_rays as u32, shots_fired);
    // }
}
