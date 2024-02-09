use macroquad::prelude as mq;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::net::UdpSocket;
use tokio::runtime::Runtime;

const WINDOW_WIDTH: u32 = 1024;
const WINDOW_HEIGHT: u32 = 512;

const MAP_WIDTH: u32 = 24;
const MAP_HEIGHT: u32 = 24;
const TILE_SIZE: f32 = 64.0 / 3.0;

const NUM_RAYS: u32 = 512;
const RAYS_PER_SECOND: f32 = NUM_RAYS as f32 / 2.0;

const FOV: f32 = std::f32::consts::PI / 2.0;

const VIEW_DISTANCE: f32 = 7.0 * TILE_SIZE;

const NUM_TEXTURES: i32 = 3;

const BACKGROUND_COLOR: mq::Color = mq::Color::new(73.0 / 255.0, 1.0, 1.0, 1.0);
const GROUND_COLOR: mq::Color = mq::Color::new(36.0 / 255.0, 219.0 / 255.0, 0.0, 1.0);
const WALL_COLOR_LIGHT: mq::Color = mq::Color::new(0.6, 0.6, 0.6, 1.0);
const WALL_COLOR_DARK: mq::Color = mq::Color::new(0.55, 0.55, 0.55, 1.0);
const NORD_COLOR: mq::Color = mq::Color::new(46.0 / 255.0, 52.0 / 255.0, 64.0 / 255.0, 1.0);

#[derive(Serialize, Deserialize, Clone)]
struct PlayerUpdate {
    id: u8,
    action: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GameState {
    players: Vec<Player>,
    map: Vec<u8>,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
struct Ray {
    pos: (f32, f32),
    angle: f32,
    direction: (f32, f32),
}
#[derive(Clone, Serialize, Deserialize, Debug)]
struct Player {
    id: u8,
    pos: (f32, f32),
    direction: (f32, f32),
    angle: f32,          // in radians
    angle_vertical: f32, // in radians
    last_input_time: f64,
    action: String,
    score: u32,
}

impl Player {
    fn draw(&self, scaling_info: &ScalingInfo) {
        mq::draw_circle(
            scaling_info.offset.x + self.pos.0 * scaling_info.width / WINDOW_WIDTH as f32,
            scaling_info.offset.y + self.pos.1 * scaling_info.height / WINDOW_HEIGHT as f32,
            8.0,
            mq::YELLOW,
        );
        mq::draw_line(
            scaling_info.offset.x + self.pos.0 * scaling_info.width / WINDOW_WIDTH as f32,
            scaling_info.offset.y + self.pos.1 * scaling_info.height / WINDOW_HEIGHT as f32,
            scaling_info.offset.x
                + self.pos.0 * scaling_info.width / WINDOW_WIDTH as f32
                + self.angle.cos() * 20.0,
            scaling_info.offset.y
                + self.pos.1 * scaling_info.height / WINDOW_HEIGHT as f32
                + self.angle.sin() * 20.0,
            3.0,
            mq::YELLOW,
        );
    }
    fn cast_rays(&self, map: &mut [u8], num_rays: u32) -> Vec<(Ray, Option<RayHit>)> {
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
                    shots_fired = true;
                }
                ray.cast_ray(map, shots_fired)
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

fn draw_map(map: &[u8], scaling_info: &ScalingInfo) {
    let scaled_size = scaling_info.width / (MAP_WIDTH as f32 * 2.0);
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let wall = map[(y * MAP_WIDTH + x) as usize];
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
        let x = self.pos.0 / TILE_SIZE; // (0.0, 8.0)
        let y = self.pos.1 / TILE_SIZE; // (0.0, 8.0)
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

                    let pos =
                        mq::Vec2::new(self.pos.0, self.pos.1) + (ray_dir * distance * TILE_SIZE);

                    let map_pos = pos / TILE_SIZE;
                    let wall_pos = map_pos - map_pos.floor();
                    let wall_coord = if x_move { wall_pos.y } else { wall_pos.x };

                    return (
                        *self,
                        Some(RayHit {
                            pos: (pos.x, pos.y),
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
    // Get server IP address from the user
    print!("Enter IP:PORT of the server you wish to connect to: ");
    io::stdout().flush().unwrap();
    let mut server_addr = String::new();
    io::stdin().read_line(&mut server_addr).unwrap();
    let server_addr = server_addr.trim(); // Remove newline character

    // Get player name
    print!("Enter Name: ");
    io::stdout().flush().unwrap();
    let mut player_name = String::new();
    io::stdin().read_line(&mut player_name).unwrap();
    let playernamecopy = player_name.clone();

    // Create a new Tokio runtime
    let runtime = Runtime::new().unwrap();

    // Create the socket and connect it to the server
    let socket = runtime.block_on(async {
        let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        socket.connect(server_addr).await.unwrap();
        println!("Connected to server at {}", server_addr);
        println!("Waiting for all the players to join...");
        socket
    });

    // Wrap the socket in Arc<Mutex<>> for sharing across threads
    let shared_socket = Arc::new(Mutex::new(socket));

    // Setup networking in a separate thread
    let (tx, rx): (Sender<Vec<GameState>>, Receiver<Vec<GameState>>) = mpsc::channel();
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
            let mut game_begun = false;

            // COMMUNICATION LOOP
            loop {
                let mut gameloopupdate = true;
                // Check for updates from the main game loop to send to the server
                let player_update = match rx_update.try_recv() {
                    Ok(update) => update,
                    Err(_) => {
                        gameloopupdate = false;
                        PlayerUpdate {
                            id: player_id.clone().parse().unwrap(),
                            action: "ping".to_string(),
                        }
                    }
                };

                if gameloopupdate {
                    let update_msg = serde_json::to_string(&player_update).unwrap();
                    // println!("Sending update to server {}", update_msg);
                    socket.send(update_msg.as_bytes()).await.unwrap();
                }

                // check if there is an update from the server
                let mut buf = [0u8; 10024];
                match socket.try_recv(&mut buf) {
                    Ok(len) => {
                        let update: GameState = serde_json::from_slice(&buf[..len]).unwrap();
                        tx.send(vec![update]).unwrap();
                        if !game_begun {
                            println!("Game has begun!");
                            game_begun = true;
                        }
                    }
                    Err(_) => {
                        //  println!("Error receiving data: {:?}", e); // Log errors
                        //we come here if there is no update from the server
                    }
                }
            }
        });
    });

    let player_id = rx_id.recv().unwrap();
    // println!("Assigned ID: {}", player_id);
    let player_id: u8 = player_id.parse().unwrap();
    let wall_image = mq::Image::from_file_with_format(
        include_bytes!("../resources/WolfensteinTextures.png"),
        Some(mq::ImageFormat::Png),
    );
    let mut num_rays = 0.0;
    let mut output_image =
        mq::Image::gen_image_color(WINDOW_WIDTH as u16 / 2, WINDOW_HEIGHT as u16, NORD_COLOR);
    let output_texture = mq::Texture2D::from_image(&output_image);
    //used for input throttling
    // let mut last_input_time = 0.0; // Tracks the last time player.input() was called
    // let input_threshold = 0.1; // 0.1 seconds between inputs, adjust as needed

    let player_update = PlayerUpdate {
        id: player_id,
        action: "ping".to_string(),
    };
    tx_update.send(player_update).unwrap();

    let mut game_state = rx.recv().unwrap();

    // GAME LOOP
    loop {
        // Listen for key presses and send the action to the communication thread
        listen_for_key_presses(tx_update.clone(), player_id);
        // Try to receive a game state update from the communication thread
        match rx.try_recv() {
            Ok(gs) => {
                //  println!("Received game state from server");
                game_state = gs;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                // No new game state available; continue with the current state
            }
            Err(e) => {
                // Handle other errors, such as a disconnection from the sending end
                println!("Error receiving game state: {:?}", e);
                break; // Or handle the error as appropriate for your game
            }
        }

        let mut map = game_state[0].map.clone();
        //match player id to the correct player
        let player = game_state[0]
            .players
            .iter()
            .find(|p| p.id == player_id)
            .unwrap()
            .clone();

        let scaling_info = ScalingInfo::new();
        let floor_level =
            (WINDOW_HEIGHT as f32 / 2.0) * (1.0 + player.angle_vertical.tan() / (FOV / 2.0).tan());
        let delta = mq::get_frame_time(); // seconds
        mq::clear_background(NORD_COLOR);
        draw_map(&map, &scaling_info);
        //used for input throttling
        // let current_time = mq::get_time();
        player.draw(&scaling_info);

        if num_rays < NUM_RAYS as f32 {
            num_rays += delta * RAYS_PER_SECOND;
        } else {
            num_rays = NUM_RAYS as f32;
        }
        let ray_touches = player.cast_rays(&mut map, num_rays as u32);

        for (i, ray_touch) in ray_touches.iter().enumerate() {
            let ray = &ray_touch.0;
            let ray_hit = &ray_touch.1;

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

                let color = if ray_hit.x_move {
                    WALL_COLOR_LIGHT
                } else {
                    WALL_COLOR_DARK
                };
                mq::draw_line(
                    scaling_info.offset.x + player.pos.0 * scaling_info.width / WINDOW_WIDTH as f32,
                    scaling_info.offset.y
                        + player.pos.1 * scaling_info.height / WINDOW_HEIGHT as f32,
                    scaling_info.offset.x
                        + ray_hit.pos.0 * scaling_info.width / WINDOW_WIDTH as f32,
                    scaling_info.offset.y
                        + ray_hit.pos.1 * scaling_info.height / WINDOW_HEIGHT as f32,
                    3.0,
                    color,
                );
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

        // crosshair
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
            mq::Color::new(1.0, 1.0, 1.0, 1.0),
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
            format!("PLAYER: {}", playernamecopy).as_str(),
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

        mq::next_frame().await
    }
}

// helper function for listening to key presses WASD left and right arrow keys and space
// if a key is pressed send the action to the server
fn listen_for_key_presses(tx_update: Sender<PlayerUpdate>, player_id: u8) {
    if mq::is_key_pressed(mq::KeyCode::W) {
        println!("W pressed");
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
