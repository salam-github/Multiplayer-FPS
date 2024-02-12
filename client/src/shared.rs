use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub enum AppState {
    StartScreen,
    PlayerNameSelect,
    MainMenu,
    BrowseServers,
    CreateServer,
    OptionsHelpMenu,
    Lobby,
    Game,
}

#[derive(Clone)]
pub struct AppStateData {
    pub current_state: AppState,
    pub servers: Vec<Server>, // Ensure Server is also public or in scope
    pub selected_server: Option<Server>,
    pub player_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameSessionInfo {
    pub player_name: String,
    pub created_servers: Vec<Server>, // Ensure Server struct has Serialize, Deserialize
    pub joined_server: Option<Server>,
    pub server_address: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub name: String,
    pub id: String, // Assuming you change Uuid to String for simplification; otherwise, ensure Uuid is serialized properly
    pub player_count: i32,
}