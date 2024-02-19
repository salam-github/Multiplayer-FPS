use crate::shared::{AppState, AppStateData, GameSessionInfo, Server};
use local_ip_address::local_ip;
use macroquad::prelude::*;
use macroquad::ui::{hash, root_ui, widgets, Skin};
use std::process::Command;
use uuid::Uuid;

fn render_back_button(
    ui: &mut macroquad::ui::Ui,
    current_state: &mut AppState,
    target_state: AppState,
) {
    let screen_center = vec2(screen_width() / 2.0, screen_height() / 2.0);
    if widgets::Button::new("Back")
        .size(vec2(200.0, 50.0))
        .position(vec2(screen_center.x - 100.0, screen_height() - 150.0))
        .ui(ui)
    {
        *current_state = target_state;
    }
}

pub async fn show_menu() -> Option<GameSessionInfo> {
    let mut app_state = AppStateData {
        current_state: AppState::StartScreen,
        servers: Vec::new(),
        selected_server: None,
        player_name: String::new(),
        input_ip: String::new(),
    };
    let skin = {
        let label_style = root_ui()
            .style_builder()
            .font(include_bytes!("./assets/MinimalPixel v2.ttf"))
            .unwrap()
            .text_color(Color::from_rgba(120, 120, 120, 255))
            .font_size(25)
            .build();

        let window_style = root_ui()
            .style_builder()
            .background(Image::from_file_with_format(
                include_bytes!("./assets/window_background_2.png"),
                None,
            ))
            .background_margin(RectOffset::new(52.0, 52.0, 52.0, 52.0))
            .margin(RectOffset::new(-30.0, 0.0, -30.0, 0.0))
            .build();

        let button_style = root_ui()
            .style_builder()
            .background(Image::from_file_with_format(
                include_bytes!("./assets/button_background_2.png"),
                None,
            ))
            .background_margin(RectOffset::new(8.0, 8.0, 8.0, 8.0))
            .background_hovered(Image::from_file_with_format(
                include_bytes!("./assets/button_hovered_background_2.png"),
                None,
            ))
            .background_clicked(Image::from_file_with_format(
                include_bytes!("./assets/button_clicked_background_2.png"),
                None,
            ))
            .font(include_bytes!("./assets/MinimalPixel v2.ttf"))
            .unwrap()
            .text_color(Color::from_rgba(180, 180, 100, 255))
            .font_size(40)
            .build();

        let checkbox_style = root_ui()
            .style_builder()
            .background(Image::from_file_with_format(
                include_bytes!("./assets/checkbox_background.png"),
                None,
            ))
            .background_hovered(Image::from_file_with_format(
                include_bytes!("./assets/checkbox_hovered_background.png"),
                None,
            ))
            .background_clicked(Image::from_file_with_format(
                include_bytes!("./assets/checkbox_clicked_background.png"),
                None,
            ))
            .build();

        let editbox_style = root_ui()
            .style_builder()
            .background(Image::from_file_with_format(
                include_bytes!("./assets/editbox_background.png"),
                None,
            ))
            .background_margin(RectOffset::new(2., 2., 2., 2.))
            .font(include_bytes!("./assets/MinimalPixel v2.ttf"))
            .unwrap()
            .text_color(Color::from_rgba(120, 120, 120, 255))
            .font_size(25)
            .build();

        let combobox_style = root_ui()
            .style_builder()
            .background(Image::from_file_with_format(
                include_bytes!("./assets/combobox_background.png"),
                None,
            ))
            .background_margin(RectOffset::new(4., 25., 6., 6.))
            .font(include_bytes!("./assets/MinimalPixel v2.ttf"))
            .unwrap()
            .text_color(Color::from_rgba(120, 120, 120, 255))
            .color(Color::from_rgba(210, 210, 210, 255))
            .font_size(25)
            .build();

        Skin {
            window_style,
            button_style,
            label_style,
            checkbox_style,
            editbox_style,
            combobox_style,
            ..root_ui().default_skin()
        }
    };

    let mut current_state = AppState::StartScreen;
    let mut player_name = String::new();
    let mut input_ip = String::new();
    let mut port = String::new();
    loop {
        clear_background(BLACK);
        root_ui().push_skin(&skin);

        match current_state {
            AppState::StartScreen => {
                let screen_center = vec2(screen_width() / 2.0, screen_height() / 2.0);
                if widgets::Button::new("Start")
                    .size(vec2(200.0, 60.0))
                    .position(vec2(screen_center.x - 100.0, screen_center.y - 20.0))
                    .ui(&mut root_ui())
                {
                    current_state = AppState::PlayerNameSelect;
                }
            }
            AppState::PlayerNameSelect => {
                let screen_center = vec2(screen_width() / 2.0, screen_height() / 2.0);
                let container_size = vec2(400.0, 200.0);
                let container_pos = vec2(
                    screen_center.x - container_size.x * 0.5,
                    screen_center.y - container_size.y * 0.5,
                );

                root_ui().window(hash!(), container_pos, container_size, |ui| {
                    let input_label = "Enter Name";
                    ui.label(None, input_label);
                    ui.input_text(hash!(input_label), "", &mut player_name);

                    if ui.button(None, "Confirm") {
                        let trimmed_name = player_name.trim();
                        if !trimmed_name.is_empty() {
                            // Update the player_name in AppStateData
                            app_state.player_name = trimmed_name.to_string();
                            current_state = AppState::MainMenu;
                        }
                    }
                });
            }

            AppState::MainMenu => {
                let screen_center = vec2(screen_width() / 2.0, screen_height() / 2.0);
                let button_x = screen_center.x - 300.0;

                if widgets::Button::new("Join Game")
                    .size(vec2(600.0, 50.0))
                    .position(vec2(button_x, screen_center.y - 60.0))
                    .ui(&mut root_ui())
                {
                    current_state = AppState::ConnectToServer;
                }

                if widgets::Button::new("Create Server")
                    .size(vec2(600.0, 50.0))
                    .position(vec2(button_x, screen_center.y))
                    .ui(&mut root_ui())
                {
                    current_state = AppState::CreateServer;
                }
                if widgets::Button::new("Controls")
                    .size(vec2(600.0, 50.0))
                    .position(vec2(button_x, screen_height() - 200.0))
                    .ui(&mut root_ui())
                {
                    current_state = AppState::Controls;
                }
            }

            AppState::CreateServer => {
                let screen_center = vec2(screen_width() / 2.0, screen_height() / 2.0);
                let container_size = vec2(400.0, 200.0);
                let container_pos = vec2(
                    screen_center.x - container_size.x * 0.5,
                    screen_center.y - container_size.y * 0.5,
                );

                root_ui().window(hash!(), container_pos, container_size, |ui| {
                    let input_label = "Select Port";
                    ui.label(None, input_label);
                    ui.input_text(hash!(input_label), "", &mut port);
                    let addr = format!("0.0.0.0:{port}");
                    let my_ip = local_ip().unwrap();
                    ui.label(None, &format!("Your IP: {}", my_ip));

                    if ui.button(None, "Confirm") && !port.is_empty() {
                        start_server(port.clone());
                        app_state.selected_server = Some(Server {
                            id: Uuid::new_v4().to_string(),
                            name: addr,
                        });
                        current_state = AppState::Game;
                        app_state.gather_session_info();
                    }
                });
                render_back_button(&mut root_ui(), &mut current_state, AppState::MainMenu);
            }

            AppState::ConnectToServer => {
                let screen_center = vec2(screen_width() / 2.0, screen_height() / 2.0);
                let container_size = vec2(400.0, 200.0);
                let container_pos = vec2(
                    screen_center.x - container_size.x * 0.5,
                    screen_center.y - container_size.y * 0.5,
                );

                // Input field for server IP
                root_ui().window(hash!(), container_pos, container_size, |ui| {
                    let input_label = "Enter IP Address";
                    ui.label(None, input_label);
                    ui.input_text(hash!(input_label), "", &mut input_ip);

                    if ui.button(None, "Confirm") {
                        let trimmed_ip = input_ip.trim();
                        if !trimmed_ip.is_empty() {
                            app_state.selected_server = Some(Server {
                                id: Uuid::new_v4().to_string(),
                                name: input_ip.clone(),
                            });
                            current_state = AppState::Game;
                            app_state.gather_session_info();
                        }
                    }
                });
                render_back_button(&mut root_ui(), &mut current_state, AppState::MainMenu);
            }

            AppState::Controls => {
                let screen_center = vec2(screen_width() / 2.0, screen_height() / 2.0);
                root_ui().window(
                    hash!(),
                    vec2(screen_center.x - 300.0, screen_center.y - 200.0),
                    vec2(600.0, 200.0),
                    |ui| {
                        ui.label(None, "Game Controls:");
                        ui.label(None, "- Use WASD keys to move.");
                        ui.label(None, "- Press 'Space' to shoot.");
                        ui.label(None, "- use ARROW keys to look around.");
                        ui.label(None, "First to 5 points wins the round.");
                        ui.label(None, "Next round starts in 5 seconds.");
                    },
                );
                render_back_button(&mut root_ui(), &mut current_state, AppState::MainMenu);
            }
            AppState::Game => {
                show_loading_screen().await;
                return Some(app_state.gather_session_info());
            }
        }
        root_ui().pop_skin();
        next_frame().await;
    }
}

impl AppStateData {
    // Method to gather session info for use in other parts of the game
    fn gather_session_info(&self) -> GameSessionInfo {
        GameSessionInfo {
            player_name: self.player_name.clone(),
            created_servers: self.servers.clone(),
            joined_server: self.selected_server.clone(),
            server_address: self
                .selected_server
                .as_ref()
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "".into()),
        }
    }
}

async fn show_loading_screen() {
    let screen_center = vec2(screen_width() / 2.0, screen_height() / 2.0);
    let container_size = vec2(160.0, 80.0);
    let container_pos = vec2(
        screen_center.x - container_size.x * 0.5,
        screen_center.y - container_size.y * 0.5,
    );

    let timer = Instant::now();
    loop {
        if timer.elapsed() > Duration::from_secs(2) {
            break;
        }
        clear_background(BLACK);

        root_ui().window(hash!(), container_pos, container_size, |ui| {
            ui.label(None, "Loading...");
        });

        next_frame().await;
    }
}

use std::env;
use std::time::{Duration, Instant};

fn start_server(addr: String) {
    // Get the current working directory
    let current_dir = env::current_dir().expect("Failed to get current working directory");

    // Navigate up one level to the parent directory
    let parent_dir = current_dir
        .parent()
        .expect("Failed to get parent directory");

    // Specify the path to the server directory
    let server_directory = parent_dir.join("server");

    // Convert the path to a string
    let server_directory_str = server_directory
        .to_str()
        .expect("Failed to convert path to string");

    // Specify the path to the script
    let script_path = "./../init_server.sh";

    // Run the script with the absolute path as an argument
    Command::new(script_path)
        .arg(server_directory_str)
        .arg(addr)
        .status()
        .expect("Failed to run the script");
}
