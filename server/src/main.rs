mod maze;
mod player;

use crate::maze::select_maze;
use crate::player::{Player, Position};
use rand::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::UdpSocket;

pub const MAZE_WIDTH: usize = 24;
pub const MAZE_HEIGHT: usize = MAZE_WIDTH;

pub const EMPTY: u8 = 0;
pub const PLAYER: u8 = 1;
pub const WALL: u8 = 2;

pub const BREAKABLE: u8 = 3;

const TILE_SIZE: f32 = 64.0 / 3.0;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PlayerUpdate {
    id: usize,
    action: String,
}
#[derive(Serialize, Deserialize)]
struct GameState {
    players: Vec<Player>,
    maze: Vec<u8>,
    round: usize,
    new_round_state: bool,
    winner: String,
}

impl GameState {
    fn new() -> Self {
        Self {
            players: Vec::new(),
            maze: select_maze(1, 1),
            round: 1,
            new_round_state: false,
            winner: String::from(""),
        }
    }
}

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:8080";
    let socket = UdpSocket::bind(addr).await.unwrap();
    println!("Server running on {}", addr);

    let mut clients: HashMap<SocketAddr, Player> = HashMap::new();
    let mut buf = [0u8; 1024];
    let mut game_state = GameState::new();
    let placeholder_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0));

    loop {
        let (len, client_addr) = match socket.try_recv_from(&mut buf) {
            Ok((len, client_addr)) => (len, client_addr),
            Err(_) => (0, placeholder_addr),
        };
        let msg = String::from_utf8_lossy(&buf[..len]);

        //bool to indicate when to send initial game state, after this we only send when a player is updated
        let mut send_initial_gs = false;

        if msg.starts_with("new_connection") {
            // Example: "new_connection:foo" -> "foo"
            let player_name = msg
                .split_once(':')
                .map(|(_, name)| name.trim())
                .unwrap_or("");

            if let std::collections::hash_map::Entry::Vacant(e) = clients.entry(client_addr) {
                send_initial_gs = true;
                let id = game_state.players.len().to_string();
                println!(
                    "New player connected with ID: {}, name: {}",
                    id, player_name
                );
                socket.send_to(id.as_bytes(), client_addr).await.unwrap();
                let mut rng = thread_rng();
                let new_pos: Position;
                loop {
                    let new_x_tile = rng.gen_range(0..MAZE_WIDTH);
                    let new_y_tile = rng.gen_range(0..MAZE_HEIGHT);
                    let idx = new_y_tile * MAZE_WIDTH + new_x_tile;

                    if game_state.maze[idx] == EMPTY {
                        // Calculate the center of the tile for the new position
                        new_pos = Position {
                            x: new_x_tile as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                            y: new_y_tile as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                        };

                        // Set the tile to 1 (player)
                        game_state.maze[idx] = PLAYER;
                        break;
                    }
                }
                let new_player =
                    Player::new(new_pos, game_state.players.len(), player_name.to_string());
                e.insert(new_player.clone());
                game_state.players.push(new_player.clone());
            }
        } else if let Ok(update) = serde_json::from_str::<PlayerUpdate>(&msg) {
            // Update players action in the vector of players
            for player in game_state.players.iter_mut() {
                if player.id == update.id && update.action != "ping" {
                    player.action = update.action.clone();
                }
            }
        }

        let mut has_a_player_moved = false;

        // Collect IDs of players that need to be repositioned
        let mut reposition_player_ids = Vec::new();

        for player in game_state.players.iter_mut() {
            if let Some(idx) = &player.input(&mut game_state.maze, &mut has_a_player_moved) {
                let x = idx % MAZE_WIDTH as u32;
                let y = idx / MAZE_HEIGHT as u32;

                // Instead of another mutable borrow here, just collect the IDs
                reposition_player_ids.push((player.id, x, y));
            }
        }

        // Apply updates based on collected IDs, avoiding double mutable borrow
        for (_player_id, x, y) in reposition_player_ids {
            let mut rng = rand::thread_rng();
            let new_pos: Position;
            loop {
                let new_x_tile = rng.gen_range(0..MAZE_WIDTH);
                let new_y_tile = rng.gen_range(0..MAZE_HEIGHT);
                let idx = new_y_tile * MAZE_WIDTH + new_x_tile;
                if game_state.maze[idx] == EMPTY {
                    // Calculate the center of the tile for the new position
                    new_pos = Position {
                        x: new_x_tile as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                        y: new_y_tile as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                    };
                    break;
                }
            }

            // Find and reposition the player including the circle on the map
            if let Some(player) = game_state
                .players
                .iter_mut()
                .find(|p| (p.pos.x / TILE_SIZE) as u32 == x && (p.pos.y / TILE_SIZE) as u32 == y)
            {
                player.pos = new_pos;
                // use the player.pos to place a tile 1 on the map
                let new_x = player.pos.x / TILE_SIZE;
                let new_y = player.pos.y / TILE_SIZE;
                let map_x = new_x as usize;
                let map_y = new_y as usize;
                let map_index = map_y * MAZE_WIDTH + map_x;
                game_state.maze[map_index] = PLAYER;
                has_a_player_moved = true;
            }
        }
        let new_round = game_state.update_level();
        //if one of the players has reached the score limit (5), start a new round

        if new_round {
            if game_state.round >= 4 {
                game_state.round = 1;
            }
            game_state.maze = select_maze(game_state.round, game_state.players.len());
            for player in game_state.players.iter_mut() {
                player.score = 0;
                game_state.new_round_state = true;
            }
            game_state.randomize_player_position();
        }

        if has_a_player_moved || send_initial_gs {
            //broadcast the game state to all clients
            let broadcast_msg = serde_json::to_string(&game_state).unwrap();
            for &addr in clients.keys() {
                socket
                    .send_to(broadcast_msg.as_bytes(), addr)
                    .await
                    .unwrap();
            }
            const DURATION_BETWEEN_LEVELS: u64 = 5;
            if new_round {
                tokio::time::sleep(std::time::Duration::from_secs(DURATION_BETWEEN_LEVELS)).await;
            }
            game_state.new_round_state = false;
            game_state.winner = String::from("");
        }
    }
}

impl GameState {
    fn update_level(&mut self) -> bool {
        for player in self.players.iter() {
            if player.score >= 5 {
                self.round += 1;
                self.winner = player.name.clone();
                return true;
            }
        }
        false
    }
    fn randomize_player_position(&mut self) {
        let mut rng = thread_rng();
        for player in self.players.iter_mut() {
            let new_pos: Position;
            loop {
                let new_x_tile = rng.gen_range(0..MAZE_WIDTH);
                let new_y_tile = rng.gen_range(0..MAZE_HEIGHT);
                let idx = new_y_tile * MAZE_WIDTH + new_x_tile;
                // Ensure the chosen position is empty
                if self.maze[idx] == EMPTY {
                    // Calculate the center of the tile for the new position
                    new_pos = Position {
                        x: new_x_tile as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                        y: new_y_tile as f32 * TILE_SIZE + TILE_SIZE / 2.0,
                    };

                    break;
                }
            }
            player.pos = new_pos;
            let new_x = player.pos.x / TILE_SIZE;
            let new_y = player.pos.y / TILE_SIZE;
            let map_x = new_x as usize;
            let map_y = new_y as usize;
            let map_index = map_y * MAZE_WIDTH + map_x;
            self.maze[map_index] = PLAYER;
        }
    }
}


