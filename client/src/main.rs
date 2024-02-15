use macroquad::prelude as mq;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::net::UdpSocket;
use tokio::runtime::Runtime;
mod menu;
mod shared;
use shared::GameSessionInfo;

const WINDOW_WIDTH: u32 = 1024;
const WINDOW_HEIGHT: u32 = 512;

const MAZE_WIDTH: usize = 24;
const MAZE_HEIGHT: usize = MAZE_WIDTH; // To ensure a square map

const TILE_SIZE: f32 = 64.0 / 3.0;

const NUM_RAYS: u32 = 512;
const RAYS_PER_SECOND: f32 = NUM_RAYS as f32 / 2.0;

const FOV: f32 = std::f32::consts::PI / 2.0;

const VIEW_DISTANCE: f32 = 7.0 * TILE_SIZE;

const NUM_TEXTURES: i32 = 3;

const BACKGROUND_COLOR: mq::Color = mq::Color::new(73.0 / 255.0, 1.0, 1.0, 1.0);
const GROUND_COLOR: mq::Color = mq::Color::new(36.0 / 255.0, 219.0 / 255.0, 0.0, 1.0);
const NORD_COLOR: mq::Color = mq::Color::new(46.0 / 255.0, 52.0 / 255.0, 64.0 / 255.0, 1.0);

#[derive(Serialize, Deserialize, Clone)]
struct PlayerUpdate {
    id: u8,
    action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Maze {
    pub width: usize,
    pub height: usize,
    pub layout: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GameState {
    players: Vec<Player>,
    maze: Vec<u8>,
    new_round_state: bool,
    winner: String,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
struct Position {
    x: f32,
    y: f32,
}

type Direction = Position;

impl Position {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}
#[derive(Clone, Copy, Serialize, Deserialize)]
struct Ray {
    pos: Position,
    angle: f32,
    direction: Direction,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
struct Player {
    id: u8,
    pos: Position,
    direction: Direction,
    angle: f32,          // in radians
    angle_vertical: f32, // in radians
    action: String,
    score: u32,
}

impl Player {
    fn draw(&self, scaling_info: &ScalingInfo) {
        mq::draw_circle(
            scaling_info.offset.x + self.pos.x * scaling_info.width / WINDOW_WIDTH as f32,
            scaling_info.offset.y + self.pos.y * scaling_info.height / WINDOW_HEIGHT as f32,
            8.0,
            mq::YELLOW,
        );

        // Draw the line representing the player's direction
        mq::draw_line(
            scaling_info.offset.x + self.pos.x * scaling_info.width / WINDOW_WIDTH as f32,
            scaling_info.offset.y + self.pos.y * scaling_info.height / WINDOW_HEIGHT as f32,
            scaling_info.offset.x
                + self.pos.x * scaling_info.width / WINDOW_WIDTH as f32
                + self.angle.cos() * 20.0,
            scaling_info.offset.y
                + self.pos.y * scaling_info.height / WINDOW_HEIGHT as f32
                + self.angle.sin() * 20.0,
            3.0,
            mq::YELLOW,
        );
    }


    fn cast_rays(&self, maze: &mut [u8], num_rays: u32) -> Vec<(Ray, Option<RayHit>)> {
        let rotation_matrix = mq::Mat2::from_angle(self.angle);

        (0..num_rays)
            .map(|i| {
                let un_rotated_direction =
                    mq::Vec2::new(1.0, (i as f32 / num_rays as f32 - 0.5) * FOV);
                let direction = rotation_matrix * un_rotated_direction;

                let ray = Ray::new(
                    Position::new(self.pos.x, self.pos.y),
                    Direction::new(direction.x, direction.y),
                );
                ray.cast_ray(maze)
            })
            .collect()
    }
}
#[derive(Clone, Copy, Serialize, Deserialize)]
struct RayHit {
    pos: Position,
    world_distance: f32,
    x_move: bool,
    wall_coord: f32, // 0-1.0 as x
    wall_type: u8,
}

trait Lerp {
    fn lerp(self, other: Self, t: f32) -> Self;
}
impl Lerp for mq::Color {
    fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        mq::Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }
}

fn draw_map(maze: &[u8], scaling_info: &ScalingInfo) {
    let scaled_size = scaling_info.width / (MAZE_WIDTH as f32 * 2.0);
    for y in 0..MAZE_HEIGHT {
        for x in 0..MAZE_WIDTH {
            let wall = maze[y * MAZE_WIDTH + x];
            let color = match wall {
                1 => mq::BLACK,
                2 => mq::RED,
                3 => mq::GREEN,
                _ => mq::BLACK,
            };
            mq::draw_rectangle(
                scaling_info.offset.x + x as f32 * scaled_size + 1.0,
                scaling_info.offset.y + y as f32 * scaled_size + 1.0,
                scaled_size - 2.0,
                scaled_size - 2.0,
                color,
            );
        }
    }
}

struct VerticalLine {
    x: i32,
    y0: i32,
    y1: i32,
}
impl VerticalLine {
    fn new(x: i32, y0: i32, y1: i32) -> Self {
        Self { x, y0, y1 }
    }
}
fn vertical_line(line: VerticalLine, output_image: &mut mq::Image, color: mq::Color) {
    let x = line.x.clamp(0, output_image.width() as i32 - 1) as u32;
    let y0 = line.y0.clamp(0, output_image.height() as i32 - 1) as u32;
    let y1 = line.y1.clamp(0, output_image.height() as i32 - 1) as u32;

    for y in y0..y1 {
        output_image.set_pixel(x, y, color);
    }
}

fn vertical_textured_line_with_fog(
    wall_line: VerticalLine,
    output_image: &mut mq::Image,
    texture: &mq::Image,
    texture_line: VerticalLine,
    fog_brightness: f32,
) {
    let draw_x = wall_line.x.clamp(0, output_image.width() as i32 - 1) as u32;
    let draw_y0 = wall_line.y0.clamp(0, output_image.height() as i32 - 1) as u32;
    let draw_y1 = wall_line.y1.clamp(0, output_image.height() as i32 - 1) as u32;

    let texture_x = texture_line.x.clamp(0, texture.width() as i32 - 1) as u32;

    let h = wall_line.y1 - wall_line.y0;
    let texture_h = texture_line.y1 - texture_line.y0;

    for y in draw_y0..draw_y1 {
        let h_ratio = texture_h as f32 / h as f32;
        let h_diff = y as i32 - wall_line.y0;
        let texture_y = (h_diff as f32 * h_ratio) as u32 + texture_line.y0 as u32;

        let color = texture.get_pixel(texture_x, texture_y);
        let color_with_fog = color.lerp(BACKGROUND_COLOR, fog_brightness);
        output_image.set_pixel(draw_x, y, color_with_fog);
    }
}

fn window_conf() -> mq::Conf {
    mq::Conf {
        window_title: "Wolf-Wars".to_owned(),
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
impl Ray {
    fn new(pos: Position, direction: Direction) -> Self {
        Self {
            pos,
            angle: direction.y.atan2(direction.x),
            direction,
        }
    }
    fn cast_ray(&self, maze: &mut [u8]) -> (Ray, Option<RayHit>) {
        // DDA algorithm
        let x = self.pos.x / TILE_SIZE; // (0.0, 8.0)
        let y = self.pos.y / TILE_SIZE; // (0.0, 8.0)
        let ray_start = mq::Vec2::new(x, y);

        let ray_dir = mq::Vec2::new(self.direction.x, self.direction.y).normalize();

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
                && map_check.x < MAZE_WIDTH as f32
                && map_check.y >= 0.0
                && map_check.y < MAZE_HEIGHT as f32
            {
                let map_index = (map_check.y * MAZE_WIDTH as f32 + map_check.x) as usize;
                let wall_type = maze[map_index];
                if wall_type != 0 {
                    let pos =
                        mq::Vec2::new(self.pos.x, self.pos.y) + (ray_dir * distance * TILE_SIZE);

                    let map_pos = pos / TILE_SIZE;
                    let wall_pos = map_pos - map_pos.floor();
                    let wall_coord = if x_move { wall_pos.y } else { wall_pos.x };

                    return (
                        *self,
                        Some(RayHit {
                            pos: Position::new(pos.x, pos.y),
                            world_distance: distance * TILE_SIZE,
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

#[macroquad::main(window_conf)]
async fn main() {
    // Show the menu and wait for it to return session info
    if let Some(session_info) = menu::show_menu().await {
        // Use the session info to start the game
        start_game(session_info).await;
    } else {
        eprintln!("Session info not provided, cannot start the game.");
    }
}

async fn start_game(game_session_info: GameSessionInfo) {
    let runtime = Runtime::new().expect("Failed to create runtime");

    let socket = runtime.block_on(async {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .expect("Failed to bind socket");
        socket
            .connect(&game_session_info.server_address)
            .await
            .expect("Failed to connect to server");
        socket
    });
    let player_name = game_session_info.player_name.clone();
    let player_name_copy = game_session_info.player_name.clone();

    // Wrap the socket in Arc<Mutex<>> for sharing across threads
    let shared_socket = Arc::new(Mutex::new(socket));
    // let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let (tx, rx): (Sender<GameState>, Receiver<GameState>) = mpsc::channel();
    let (tx_id, rx_id): (Sender<String>, Receiver<String>) = mpsc::channel();
    let (tx_update, rx_update): (Sender<PlayerUpdate>, Receiver<PlayerUpdate>) = mpsc::channel();

    let socket_clone = shared_socket.clone();

    thread::spawn(move || {
        runtime.block_on(async {
            let socket = socket_clone.lock().unwrap();

            //format a string "new_connection:{player_name}"
            let initial_msg = format!("new_connection:{}", player_name.clone().trim());

            // Send an initial message to the server to indicate a new connection
            // let initial_msg = "new_connection";
            socket.send(initial_msg.as_bytes()).await.unwrap();

            // Receive the player ID from the server
            let mut buf = [0; 1024];
            let len = socket.recv(&mut buf).await.unwrap();
            let player_id = String::from_utf8_lossy(&buf[..len]).to_string();
            tx_id.send(player_id.clone()).unwrap();

            // COMMUNICATION LOOP
            loop {
                let mut game_loop_update = true;
                // Check for updates from the main game loop to send to the server
                let player_update = match rx_update.try_recv() {
                    Ok(update) => update,
                    Err(_) => {
                        game_loop_update = false;
                        PlayerUpdate {
                            id: player_id.clone().parse().unwrap(),
                            action: "ping".to_string(),
                        }
                    }
                };

                if game_loop_update {
                    let update_msg = serde_json::to_string(&player_update).unwrap();
                    socket.send(update_msg.as_bytes()).await.unwrap();
                }

                const BUFFER_SIZE: usize = 10240;
                // check if there is an update from the server
                let mut buf = [0; BUFFER_SIZE];
                if let Ok(len) = socket.try_recv(&mut buf) {
                    let update: GameState = serde_json::from_slice(&buf[..len]).unwrap();
                    tx.send(update).unwrap(); // If tx expects GameState
                }
            }
        });
    });

    let player_id = rx_id.recv().unwrap();
    let player_id: u8 = player_id.parse().unwrap();
    let wall_image = mq::Image::from_file_with_format(
        include_bytes!("../resources/WolfensteinTextures.png"),
        Some(mq::ImageFormat::Png),
    );
    let mut num_rays = 0.0;
    let mut output_image =
        mq::Image::gen_image_color(WINDOW_WIDTH as u16 / 2, WINDOW_HEIGHT as u16, NORD_COLOR);
    let output_texture = mq::Texture2D::from_image(&output_image);

    let player_update = PlayerUpdate {
        id: player_id,
        action: "ping".to_string(),
    };
    tx_update.send(player_update).unwrap();

    let mut game_state = rx.recv().unwrap();

    loop {
        // Listen for key presses and send the action to the communication thread
        listen_for_key_presses(tx_update.clone(), player_id);
        // Try to receive a game state update from the communication thread
        match rx.try_recv() {
            Ok(gs) => {
                game_state = gs;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(e) => {
                eprintln!("Error receiving game state: {:?}", e);
                break;
            }
        }

        //match player id to the correct player
        let player = game_state
            .players
            .iter()
            .find(|p| p.id == player_id)
            .unwrap()
            .clone();
        let scaling_info = ScalingInfo::new();
        let floor_level =
            (WINDOW_HEIGHT as f32 / 2.0) * (1.0 + player.angle_vertical.tan() / (FOV / 2.0).tan());
        let delta = mq::get_frame_time();
        mq::clear_background(NORD_COLOR);
        draw_map(&game_state.maze, &scaling_info);
        player.draw(&scaling_info);

        if num_rays < NUM_RAYS as f32 {
            num_rays += delta * RAYS_PER_SECOND;
        } else {
            num_rays = NUM_RAYS as f32;
        }
        let ray_touches = player.cast_rays(&mut game_state.maze, num_rays as u32);

        for (i, (ray, ray_hit)) in ray_touches.iter().enumerate() {
            let x = i as i32;

            if let Some(ray_hit) = ray_hit {
                let angle_between = player.angle - ray.angle;
                let z = ray_hit.world_distance * angle_between.cos();

                let projection_dist = (TILE_SIZE / 2.0) / (FOV / 2.0).tan();

                let h = (WINDOW_HEIGHT as f32 * projection_dist) / z;
                let y0 = floor_level - (h / 2.0);
                let y1 = y0 + h;

                let y0 = y0.round() as i32;
                let y1 = y1.round() as i32;

                let texture_x = (ray_hit.wall_coord * wall_image.width() as f32).round() as i32;
                let texture_y0 =
                    (wall_image.height() as i32 / NUM_TEXTURES) * (ray_hit.wall_type as i32 - 1);
                let texture_y1 = texture_y0 + wall_image.height() as i32 / NUM_TEXTURES;

                let sky = VerticalLine::new(x, 0, y0);
                vertical_line(sky, &mut output_image, BACKGROUND_COLOR);

                let fog_brightness = (2.0 * ray_hit.world_distance / VIEW_DISTANCE - 1.0).max(0.0);

                let wall_line = VerticalLine::new(x, y0, y1);
                let texture_line = VerticalLine::new(texture_x, texture_y0, texture_y1);
                vertical_textured_line_with_fog(
                    wall_line,
                    &mut output_image,
                    &wall_image,
                    texture_line,
                    fog_brightness,
                );

                let floor = VerticalLine::new(x, y1, WINDOW_HEIGHT as i32);
                vertical_line(floor, &mut output_image, GROUND_COLOR);
            } else {
                let floor_y = floor_level.round() as i32;

                let sky = VerticalLine::new(x, 0, floor_y);
                vertical_line(sky, &mut output_image, BACKGROUND_COLOR);

                let floor = VerticalLine::new(x, floor_y, WINDOW_HEIGHT as i32);
                vertical_line(floor, &mut output_image, GROUND_COLOR);
            }
        }

        output_texture.update(&output_image);

        mq::draw_texture_ex(
            output_texture,
            scaling_info.offset.x + scaling_info.width / 2.0,
            scaling_info.offset.y,
            mq::WHITE,
            mq::DrawTextureParams {
                dest_size: Some(mq::Vec2::new(
                    scaling_info.width / 2.0,
                    scaling_info.height + 1.0,
                )),
                ..Default::default()
            },
        );

        // cross-hair
        mq::draw_line(
            scaling_info.offset.x + scaling_info.width * (3.0 / 4.0) - 10.0,
            scaling_info.offset.y + scaling_info.height / 2.0,
            scaling_info.offset.x + scaling_info.width * (3.0 / 4.0) + 10.0,
            scaling_info.offset.y + scaling_info.height / 2.0,
            2.0,
            mq::BLACK,
        );
        mq::draw_line(
            scaling_info.offset.x + scaling_info.width * (3.0 / 4.0),
            scaling_info.offset.y + scaling_info.height / 2.0 - 10.0,
            scaling_info.offset.x + scaling_info.width * (3.0 / 4.0),
            scaling_info.offset.y + scaling_info.height / 2.0 + 10.0,
            2.0,
            mq::BLACK,
        );

        // text background
        mq::draw_rectangle(
            scaling_info.offset.x + 1.0,
            scaling_info.offset.y + 1.0,
            140.0,
            50.0,
            mq::Color::new(1.0, 1.0, 1.0, 0.5),
        );

        // text
        mq::draw_text(
            format!("FPS: {}", mq::get_fps()).as_str(),
            scaling_info.offset.x + 5.,
            scaling_info.offset.y + 15.,
            20.,
            mq::BLUE,
        );
        mq::draw_text(
            format!("PLAYER: {}", player_name_copy).as_str(),
            scaling_info.offset.x + 5.,
            scaling_info.offset.y + 30.,
            20.,
            mq::BLUE,
        );
        mq::draw_text(
            format!("Score: {}", player.score).as_str(),
            scaling_info.offset.x + 5.,
            scaling_info.offset.y + 45.,
            20.,
            mq::BLUE,
        );

        if game_state.new_round_state {
            mq::draw_text(
                format!("WINNER IS {}", game_state.winner).as_str(),
                scaling_info.offset.x + 300.,
                scaling_info.offset.y + 250.,
                50.,
                mq::BLUE,
            );
            mq::draw_text(
                "VI BÖRJÄR NYA ROUND MOTHERFUCKERS",
                scaling_info.offset.x + 300.,
                scaling_info.offset.y + 300.,
                50.,
                mq::BLUE,
            );
        }
        mq::next_frame().await
    }
}

// helper function for listening to key presses WASD left and right arrow keys and space
// if a key is pressed send the action to the server
fn listen_for_key_presses(tx_update: Sender<PlayerUpdate>, player_id: u8) {
    if mq::is_key_pressed(mq::KeyCode::W) {
        let player_update = PlayerUpdate {
            id: player_id,
            action: "W".to_string(),
        };
        tx_update.send(player_update).unwrap();
    }
    if mq::is_key_pressed(mq::KeyCode::A) {
        let player_update = PlayerUpdate {
            id: player_id,
            action: "A".to_string(),
        };
        tx_update.send(player_update).unwrap();
    }
    if mq::is_key_pressed(mq::KeyCode::S) {
        let player_update = PlayerUpdate {
            id: player_id,
            action: "S".to_string(),
        };
        tx_update.send(player_update).unwrap();
    }
    if mq::is_key_pressed(mq::KeyCode::D) {
        let player_update = PlayerUpdate {
            id: player_id,
            action: "D".to_string(),
        };
        tx_update.send(player_update).unwrap();
    }
    if mq::is_key_pressed(mq::KeyCode::Left) {
        let player_update = PlayerUpdate {
            id: player_id,
            action: "left".to_string(),
        };
        tx_update.send(player_update).unwrap();
    }
    if mq::is_key_pressed(mq::KeyCode::Right) {
        let player_update = PlayerUpdate {
            id: player_id,
            action: "right".to_string(),
        };
        tx_update.send(player_update).unwrap();
    }
    if mq::is_key_pressed(mq::KeyCode::Space) {
        let player_update = PlayerUpdate {
            id: player_id,
            action: "shoot".to_string(),
        };
        tx_update.send(player_update).unwrap();
    }
}
