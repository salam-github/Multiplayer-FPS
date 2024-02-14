use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum AppState {
    StartScreen,
    PlayerNameSelect,
    MainMenu,
    ConnectToServer,
    CreateServer,
    OptionsHelpMenu,
    Lobby,
    Game,
}

#[derive(Clone)]
pub struct AppStateData {
    pub current_state: AppState,
    pub servers: Vec<Server>,
    pub selected_server: Option<Server>,
    pub player_name: String,
    pub input_ip: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameSessionInfo {
    pub player_name: String,
    pub created_servers: Vec<Server>,
    pub joined_server: Option<Server>,
    pub server_address: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub name: String,
    pub id: String,
    pub player_count: i32,
}
