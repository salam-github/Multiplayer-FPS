use macroquad::prelude as mq;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::UdpSocket;

const WINDOW_WIDTH: u32 = 1024;
const WINDOW_HEIGHT: u32 = 512;

const MAP_WIDTH: u32 = 8;
const MAP_HEIGHT: u32 = 8;
const TILE_SIZE: u32 = 64;

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
#[derive(Clone, Serialize, Deserialize)]
struct Player {
    id: u8,
    pos: (f32, f32),
    direction: (f32, f32),
    angle: f32,          // in radians
    angle_vertical: f32, // in radians
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
            last_input_time: 0.0,
            action: String::from(""),
        }
    }
    fn touching_wall(&mut self, move_vec: mq::Vec2, map: &[u8], moved: &mut bool) {
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
            self.action = String::from("");
            *moved = true;
        }
    }
    fn input(&mut self, map: &[u8], moved: &mut bool) {
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
        } else if self.pos.0 > MAP_WIDTH as f32 * TILE_SIZE as f32 {
            self.pos.0 = MAP_WIDTH as f32 * TILE_SIZE as f32;
        }

        if self.pos.1 < 0.0 {
            self.pos.1 = 0.0;
        } else if self.pos.1 > MAP_HEIGHT as f32 * TILE_SIZE as f32 {
            self.pos.1 = MAP_HEIGHT as f32 * TILE_SIZE as f32;
        }
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

        if msg == "new_connection" {
            if !clients.contains_key(&client_addr) {
                player_count += 1;
                let id = player_count.to_string();
                println!("New player connected: {}", id);
                send_initial_gs = true;
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
                players.push(new_player.clone());
            }
        } else if let Ok(update) = serde_json::from_str::<PlayerUpdate>(&msg) {
            println!("Received action update from {}", client_addr);
            // println!("update: {:?}", update.action);
            // println!("update id: {:?}", update);
            // Update players action in the vector of players
            for player in players.iter_mut() {
                println!("player id in the players struct: {:?}", player.id);
                if player.id == update.id && update.action != "ping" {
                    player.action = update.action.clone();
                    println!("player action updated: {:?}", player.action);
                }
            }
        }

        //add a wall type 3 to the map at the player's position
        // for player in players.iter() {
        //     let map_x = (player.pos.0 / TILE_SIZE as f32) as usize;
        //     let map_y = (player.pos.1 / TILE_SIZE as f32) as usize;
        //     let map_index = map_y * MAP_WIDTH as usize + map_x;
        //     map[map_index] = 3;
        // }

        let mut has_a_player_moved = false;

        for player in players.iter_mut() {
            //used for input throttling
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
            // Check if enough time has elapsed since the last input
            if current_time - player.last_input_time >= input_threshold {
                player.input(&map, &mut has_a_player_moved);
                player.last_input_time = current_time; // Update the last input time
            }
        }
        //if a player has moved, update the game state
        if has_a_player_moved || send_initial_gs {
            println!("Player has moved or new player connected, sending game state to all clients");

            gamestate.players = players.clone();
            //broadcast the game state to all clients
            let broadcast_msg = serde_json::to_string(&gamestate).unwrap();
            for &addr in clients.keys() {
                println!("Sending update to {}", addr);
                //  println!("broadcast_msg: {:?}", broadcast_msg);
                socket
                    .send_to(broadcast_msg.as_bytes(), addr)
                    .await
                    .unwrap();
            }
        }
        //sleep for a bit
        // tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}
