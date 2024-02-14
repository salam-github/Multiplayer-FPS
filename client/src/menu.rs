use crate::shared::{AppState, AppStateData, GameSessionInfo, Server};
use macroquad::prelude::*;
use macroquad::ui::{hash, root_ui, widgets, Skin};
use serde::{Deserialize, Serialize};
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
        .position(vec2(screen_center.x - 100.0, screen_height() - 80.0))
        .ui(ui)
    {
        println!("Back button clicked, transitioning to MainMenu");
        *current_state = target_state;
    }
}




const PLAYER_COUNT: i32 = 10;

// #[macroquad::main("Haze wars")]
pub async fn show_menu() -> Option<GameSessionInfo> {
    let mut session_info: Option<GameSessionInfo> = None;

    let mut app_state = AppStateData {
        current_state: AppState::StartScreen,
        servers: Vec::new(),
        selected_server: None,
        player_name: String::new(),
        input_ip: String::new(),
        // is_fullscreen: false,
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
    let mut server_name = String::new();
    let mut is_fullscreen = false;
    let mut input_ip = String::new();

    loop {
        clear_background(BLACK);
        root_ui().push_skin(&skin);

        match current_state {
            AppState::StartScreen => {
                let screen_center = vec2(screen_width() / 2.0, screen_height() / 2.0);
                if widgets::Button::new("Start")
                    .size(vec2(200.0, 60.0))
                    .position(vec2(screen_center.x - 100.0, screen_center.y - 20.0))
                    .ui(&mut *root_ui())
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
                            println!("Player Name: {}", app_state.player_name);
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
                    .ui(&mut *root_ui())
                {
                    current_state = AppState::ConnectToServer;
                }
                if widgets::Button::new("Create Server")
                    .size(vec2(600.0, 50.0))
                    .position(vec2(button_x, screen_center.y - 10.0))
                    .ui(&mut *root_ui())
                {
                    current_state = AppState::CreateServer;
                }
                if widgets::Button::new("Options/Help")
                    .size(vec2(600.0, 50.0))
                    .position(vec2(button_x, screen_height() - 60.0))
                    .ui(&mut *root_ui())
                {
                    current_state = AppState::OptionsHelpMenu;
                } //remove debug info button before release
                if widgets::Button::new("Debug Info")
                    .size(vec2(600.0, 50.0))
                    .position(vec2(button_x, screen_height() - 120.0))
                    .ui(&mut *root_ui())
                {
                    let session_info = app_state.gather_session_info();
                    println!("Debug Info: \n{}", session_info.to_debug_string());
                }
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
                            // Update the input_id in AppStateData
                            println!("Attempting to connect to server: {}", trimmed_ip);
                            app_state.selected_server = Some(Server {
                                id: Uuid::new_v4().to_string(),
                                name: input_ip.clone(),
                                player_count: PLAYER_COUNT,
                            });
                            current_state = AppState::Lobby;
                        }
                    }
                });
            
                // Optionally display recent connections if needed
                // Example: Displaying below the input field, adjust positions as necessary
                // for (index, server) in app_state.servers.iter().enumerate() {
                //     if widgets::Button::new(&*server.name)
                //         .size(vec2(300.0, 30.0))
                //         .position(vec2(
                //             screen_center.x - 150.0,
                //             100.0 + index as f32 * 35.0, // Start below the input field
                //         ))
                //         .ui(&mut *root_ui())
                //     {
                //         println!("Selected recent connection: {}", server.name);
                //         app_state.selected_server = Some(server.clone());
                //         // You could directly attempt to connect here or just set the selected_server
                //     }
                // }
            
                render_back_button(&mut *root_ui(), &mut current_state, AppState::MainMenu);
            }
            

            AppState::CreateServer => {
                let screen_center = vec2(screen_width() / 2.0, screen_height() / 2.0);
                let container_size = vec2(500.0, 300.0);
                let container_pos = vec2(
                    screen_center.x - container_size.x * 0.5,
                    screen_center.y - container_size.y * 0.5,
                );


                root_ui().window(hash!(), container_pos, container_size, |ui| {
                    let input_label = "Server Name/IP:";
                    ui.label(None, input_label);
                    ui.input_text(hash!(input_label), "", &mut server_name);

                    if ui.button(None, "Create New Server") {
                        if !server_name.trim().is_empty() {
                            // Proceed with server creation only if the server name is not empty
                            let server_id = Uuid::new_v4(); // Generate a unique server ID
                            let new_server = Server {
                                id: server_id.to_string(),
                                name: server_name.trim().to_string(),
                                player_count: PLAYER_COUNT,
                                // address: "".to_string(),
                            };
                            println!("Server Created: {} ID: {}", server_name, server_id);
                            app_state.servers.push(new_server.clone());
                            app_state.selected_server = Some(new_server);
                            current_state = AppState::Lobby;
                        }
                    }
                    
                });
                render_back_button(&mut *root_ui(), &mut current_state, AppState::MainMenu);
            }

            AppState::Lobby => {
                if let Some(ref server) = app_state.selected_server {
                    let screen_center = vec2(screen_width() / 2.0, screen_height() / 2.0);

                    root_ui().window(
                        hash!(),
                        vec2(screen_center.x - 250.0, screen_center.y - 150.0),
                        vec2(500.0, 300.0),
                        |ui| {
                            ui.label(None, &format!("Server: {}", server.name));
                            // Use hardcoded player count for now
                            ui.label(None, &format!("Players: {}", PLAYER_COUNT));

                            if ui.button(None, "Join Game") {
                                println!(
                                    "{} Joining game on server: {} with {} players",
                                    app_state.player_name, server.name, PLAYER_COUNT
                                );

                                // Assuming you have a function to serialize and send or start the backend process
                                let session_info = GameSessionInfo {
                                    player_name: app_state.player_name.clone(),
                                    created_servers: app_state.servers.clone(),
                                    joined_server: Some(server.clone()),
                                    server_address: server.name.clone(),
                                };

                                // run_backend_process(&session_info);
                                let serialized_data = serde_json::to_string(&session_info)
                                    .expect("Failed to serialize");
                                println!("Serialized session info: {}", serialized_data);

                                // Transition to the game state or perform other setup as necessary
                                current_state = AppState::Game;
                                Some(app_state.gather_session_info());
                            }
                        },
                    );
                } else {
                    println!("Error: No server selected");
                    app_state.current_state = AppState::ConnectToServer;
                }
                render_back_button(&mut *root_ui(), &mut current_state, AppState::MainMenu);
            }

            AppState::OptionsHelpMenu => {
                let screen_center = vec2(screen_width() / 2.0, screen_height() / 2.0);
                root_ui().window(
                    hash!(),
                    vec2(screen_center.x - 300.0, screen_center.y - 200.0),
                    vec2(600.0, 400.0),
                    |ui| {
                        // Placeholder text for game control instructions
                        ui.label(None, "Game Controls:");
                        ui.label(None, "- Use WASD keys to move.");
                        ui.label(None, "- Press 'Space' to shoot.");
                        ui.label(None, "- use ARROW keys to look around.");

                        // Checkbox for fullscreen control
                        ui.checkbox(
                            hash!(),
                            "Fullscreen(WIP)",
                            &mut is_fullscreen, // variable to hold fullscreen state
                        );
                    },
                );
                render_back_button(&mut *root_ui(), &mut current_state, AppState::MainMenu);
            } // Add additional states if necessary
            AppState::Game => {
                println!("Transitioning to game...");
                // Prepare the session_info based on user choices in the menu
                session_info = Some(app_state.gather_session_info());
                break; // Break out of the loop to return session_info
            }
        }
        root_ui().pop_skin();

        next_frame().await;
    }
    session_info
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
impl GameSessionInfo {
    // Method to format the information into a string
    fn to_debug_string(&self) -> String {
        let servers = self
            .created_servers
            .iter()
            .map(|s| format!("{} (ID: {}, Players: {})", s.name, s.id, s.player_count))
            .collect::<Vec<_>>()
            .join(", ");
        let selected_server = self
            .joined_server
            .as_ref()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "None".into());

        format!(
            "Player Name: {}\nCreated Servers: {}\nSelected Server: {}",
            self.player_name, servers, selected_server
        )
    }
}
// Ensure this is within a function
fn run_backend_process(session_info: &GameSessionInfo) {
    // Serialize session_info within the function scope
    let serialized_data = serde_json::to_string(session_info).expect("Failed to serialize");

    // Now you can use serialized_data to start the backend process
    std::process::Command::new("../maze-wars/target/release/maze-wars")
        .arg(&serialized_data)
        .spawn()
        .expect("Failed to start the backend server executable");
}
