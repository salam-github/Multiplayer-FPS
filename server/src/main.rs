use macroquad::prelude as mq;
use rand::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::UdpSocket;

const MAP_WIDTH: u32 = 24;
const MAP_HEIGHT: u32 = 24;
const TILE_SIZE: f32 = 64.0 / 3.0;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PlayerUpdate {
    id: u8,
    action: String,
}
#[derive(Serialize, Deserialize)]
struct GameState {
    players: Vec<Player>,
    map: Vec<u8>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
struct Player {
    id: u8,
    pos: (f32, f32),
    direction: (f32, f32),
    angle: f32,          // in radians
    angle_vertical: f32, // in radians
    action: String,
    name: String,
    score: u32,
}
impl Player {
    fn new(pos: (f32, f32), id: u8, name: String) -> Self {
        Self {
            id,
            pos,
            direction: (1.0_f32, 0.0_f32),
            angle: 0.0,
            angle_vertical: 0.0,
            action: String::from(""),
            name,
            score: 0,
        }
    }
    fn touching_wall(&mut self, move_vec: mq::Vec2, map: &mut [u8], moved: &mut bool) {
        let new_x = self.pos.0 + TILE_SIZE * move_vec.x;
        let new_y = self.pos.1 + TILE_SIZE * move_vec.y;

        let map_x = (new_x / TILE_SIZE) as usize;
        let map_y = (new_y / TILE_SIZE) as usize;
        let map_index = map_y * MAP_WIDTH as usize + map_x;

        if map[map_index] == 0 {
            // Assuming 0 is an empty tile
            //set the current positions tile to 0
            let current_map_x = (self.pos.0 / TILE_SIZE) as usize;
            let current_map_y = (self.pos.1 / TILE_SIZE) as usize;
            let current_map_index = current_map_y * MAP_WIDTH as usize + current_map_x;
            map[current_map_index] = 0;
            self.pos.0 = new_x;
            self.pos.1 = new_y;
            //set the new position to 1 (player)
            map[map_index] = 1;
            self.action = String::from("");
            *moved = true;
        }
    }
    fn input(&mut self, map: &mut [u8], moved: &mut bool) -> Option<u32> {
        // Updated so you turn 90 degrees at a time
        if self.action == "left" {
            self.angle -= std::f32::consts::FRAC_PI_2;
            self.action = String::from("");
            //Set moved to true
            *moved = true;
        }
        if self.action == "right" {
            self.angle += std::f32::consts::FRAC_PI_2;
            self.action = String::from("");
            //Set moved to true
            *moved = true;
        }

        if self.action == "shoot" {
            // Convert player position to grid coordinates
            let grid_x = (self.pos.0 / TILE_SIZE).floor() as usize;
            let grid_y = (self.pos.1 / TILE_SIZE).floor() as usize;

            // Determine direction to step through the map based on angle
            let step_x = self.angle.cos().round() as isize; // Round to ensure we move strictly in grid directions
            let step_y = self.angle.sin().round() as isize;

            // Initialize variables for iteration
            let mut current_x = grid_x as isize;
            let mut current_y = grid_y as isize;
            // let mut tile_found = false;
            //make sure we start from the tile next to the player, not from the player's tile
            current_x += step_x;
            current_y += step_y;

            // Iterate through the map until we find a wall of type '3' or reach the edge
            while current_x >= 0
                && current_x < MAP_WIDTH as isize
                && current_y >= 0
                && current_y < MAP_HEIGHT as isize
            {
                let idx = (current_y * MAP_WIDTH as isize + current_x) as usize;
                //if idx is 2 return none and break
                if map[idx] == 2 {
                    break;
                }

                if map[idx] == 1 || map[idx] == 3 {
                    // Assuming '3' is the byte value representing the wall type
                    // tile_found = true;
                    if map[idx] == 1 {
                        self.score += 1;
                    }
                    //remove the wall
                    map[idx] = 0;

                    //set moved to true
                    *moved = true;
                    //reset action
                    self.action = String::from("");
                    return Some(idx as u32);
                }
                // Move to the next tile in the direction
                current_x += step_x;
                current_y += step_y;
            }

            self.action = String::from(""); // Clear action after processing
        }

        self.direction = (self.angle.cos(), self.angle.sin());

        let mut move_vec = mq::Vec2::new(0.0, 0.0);
        // Updated so you move one tile at a time

        if self.action == "W" {
            move_vec = mq::Vec2::new(self.direction.0, self.direction.1);
        }
        if self.action == "S" {
            move_vec = mq::Vec2::new(-self.direction.0, -self.direction.1);
        }
        if self.action == "D" {
            move_vec = mq::Vec2::new(-self.direction.1, self.direction.0);
        }
        if self.action == "A" {
            move_vec = mq::Vec2::new(self.direction.1, -self.direction.0);
        }

        if move_vec.length() > 0.0 {
            self.touching_wall(move_vec, map, moved);
        }

        if self.pos.0 < 0.0 {
            self.pos.0 = 0.0;
        } else if self.pos.0 > MAP_WIDTH as f32 * TILE_SIZE {
            self.pos.0 = MAP_WIDTH as f32 * TILE_SIZE;
        }

        if self.pos.1 < 0.0 {
            self.pos.1 = 0.0;
        } else if self.pos.1 > MAP_HEIGHT as f32 * TILE_SIZE {
            self.pos.1 = MAP_HEIGHT as f32 * TILE_SIZE;
        }
        None
    }
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let socket = UdpSocket::bind(addr).await.unwrap();
    println!("Server running on {}", addr);

    let mut clients: HashMap<SocketAddr, Player> = HashMap::new();
    let mut player_count: usize = 0;
    let mut buf = [0u8; 1024];
    let mut players: Vec<Player> = Vec::new();

    #[rustfmt::skip]
    let mut map = [
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 3, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 3, 0, 2, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 3, 3, 3, 2, 2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 3, 3, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 3, 0, 3, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 3, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 3, 3, 0, 0, 3, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 3, 0, 3, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 2,
            2, 3, 3, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        ];
    let mut gamestate = GameState {
        players: Vec::new(),
        map: map.to_vec(),
    };

    let placeholder_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0));

    loop {
        let (len, client_addr) = match socket.try_recv_from(&mut buf) {
            Ok((len, client_addr)) => (len, client_addr),
            Err(_) => {
                //  println!("No message received");
                //return placeholder shit
                (0, placeholder_addr)
            }
        };
        let msg = String::from_utf8_lossy(&buf[..len]);

        //bool to indicate when to send initial game state, after this we only send when a player is updated
        let mut send_initial_gs = false;

        if msg.contains("new_connection") {
            //Retrieve name from incoming msg that is "new_connection:{player_name}"
            let player_name = msg
                .split_once(':')
                .map(|(_, name)| name.trim())
                .unwrap_or("");
            //this condition below is a more efficient way than doing a .contains_key on a hashmap
            if let std::collections::hash_map::Entry::Vacant(e) = clients.entry(client_addr) {
                player_count += 1;
                let id = player_count.to_string();
                println!(
                    "New player connected with ID: {}, name: {}",
                    id, player_name
                );
                if player_count == 2 {
                    send_initial_gs = true;
                }
                socket.send_to(id.as_bytes(), client_addr).await.unwrap();
                //randomize the player's position on the map and make sure it's not on a wall
                let mut rng = rand::thread_rng();
                let new_pos: (f32, f32);
                loop {
                    let new_x_tile = rng.gen_range(0..MAP_WIDTH) as usize;
                    let new_y_tile = rng.gen_range(0..MAP_HEIGHT) as usize;
                    let idx = new_y_tile * MAP_WIDTH as usize + new_x_tile;
                    // Ensure the chosen position is empty
                    if map[idx] == 0 {
                        // Calculate the center of the tile for the new position
                        new_pos = (
                            new_x_tile as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                            new_y_tile as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                        );
                        // Set the tile to 1 (player)
                        map[idx] = 1;
                        break;
                    }
                }
                let new_player = Player::new(new_pos, player_count as u8, player_name.to_string());
                e.insert(new_player.clone());
                players.push(new_player.clone());
            }
        } else if let Ok(update) = serde_json::from_str::<PlayerUpdate>(&msg) {
            // Update players action in the vector of players
            for player in players.iter_mut() {
                if player.id == update.id && update.action != "ping" {
                    player.action = update.action.clone();
                }
            }
        }

        let mut has_a_player_moved = false;

        // Collect IDs of players that need to be repositioned
        let mut reposition_player_ids = Vec::new();

        for player in players.iter_mut() {
            if let Some(idx) = &player.input(&mut map, &mut has_a_player_moved) {
                let x = idx % MAP_WIDTH;
                let y = idx / MAP_WIDTH;

                // Instead of another mutable borrow here, just collect the IDs
                reposition_player_ids.push((player.id, x, y));
            }
        }

        // Apply updates based on collected IDs, avoiding double mutable borrow
        for (_player_id, x, y) in reposition_player_ids {
            let mut rng = rand::thread_rng();
            let new_pos: (f32, f32);
            loop {
                let new_x_tile = rng.gen_range(0..MAP_WIDTH) as usize;
                let new_y_tile = rng.gen_range(0..MAP_HEIGHT) as usize;
                let idx = new_y_tile * MAP_WIDTH as usize + new_x_tile;
                // Ensure the chosen position is empty
                if map[idx] == 0 {
                    // Calculate the center of the tile for the new position
                    new_pos = (
                        new_x_tile as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                        new_y_tile as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                    );
                    break;
                }
            }

            // Find and reposition the player including the circle on the map
            if let Some(player) = players
                .iter_mut()
                .find(|p| (p.pos.0 / TILE_SIZE) as u32 == x && (p.pos.1 / TILE_SIZE) as u32 == y)
            {
                player.pos = new_pos;
                // use the player.pos to place a tile 1 on the map
                let new_x = player.pos.0 / TILE_SIZE;
                let new_y = player.pos.1 / TILE_SIZE;
                let map_x = new_x as usize;
                let map_y = new_y as usize;
                let map_index = map_y * MAP_WIDTH as usize + map_x;
                map[map_index] = 1;
                has_a_player_moved = true;
            }
        }

        gamestate.map = map.to_vec();
        //if a player has moved, update the game state
        if has_a_player_moved || send_initial_gs {
            // println!(
            //     "Player has moved or enough players connected, sending game state to all clients"
            // );

            gamestate.players = players.clone();
            //broadcast the game state to all clients
            let broadcast_msg = serde_json::to_string(&gamestate).unwrap();
            for &addr in clients.keys() {
                // println!("Sending update to {}", addr);
                // println!("broadcast_msg: {:?}", broadcast_msg);
                socket
                    .send_to(broadcast_msg.as_bytes(), addr)
                    .await
                    .unwrap();
            }
        }
    }
}
