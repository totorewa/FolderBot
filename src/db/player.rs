use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Player {
    // Basic player metadata
    pub username: String,
    pub files: i64, // player's currency
    pub last_message: u64, // last message time THAT ADDED FILES.

    // Tracking metadata :)
    pub sent_messages: u64,
    pub sent_commands: u64,

    // Trident metadata
    pub trident_acc: u64,
    pub max_trident: u64,
    pub tridents_rolled: u64,
}

impl std::fmt::Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} files, {} messages sent, {} commands sent, {:.2} average trident rolled out of {} rolls", &self.username, self.files, self.sent_messages, self.sent_commands, self.average_trident(), self.tridents_rolled)
    }
}

pub struct PlayerData {
    pub players: HashMap<String, Player>,
}

impl Player {
    pub fn new(name: String) -> Player {
        Player {
            username: name,
            files: 1000,
            ..Default::default()
        }
    }

    pub fn average_trident(&self) -> f64 {
        if self.tridents_rolled == 0 { 0.0 }
        else { self.trident_acc as f64 / self.tridents_rolled as f64 }
    }
}

impl PlayerData {
    pub fn new() -> PlayerData {
        PlayerData {
            players: get_players(&Path::new("v2_players.json")),
        }
    }

    pub fn save(&self) {
        let _ = save_players(&self.players, &Path::new("v2_players.json"));
    }

    pub fn player(&mut self, name: &String) -> &mut Player {
        self.players.entry(name.clone()).or_insert_with(|| Player::new(name.clone()))
    }
}

impl Drop for PlayerData {
    fn drop(&mut self) {
        self.save()
    }
}

pub fn save_players(val: &HashMap<String, Player>, path: &Path) -> bool {
    let file = match match path.exists() {
        true => std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path),
        false => File::create(path),
    } {
        Ok(file) => file,
        Err(e) => {
            println!("Could not open or create file! {}", e);
            return false;
        }
    };
    match serde_json::to_writer_pretty(file, val) {
        Ok(_) => true,
        Err(e) => {
            println!("Couldn't save players: {}", e);
            false
        }
    }
}

pub fn get_players(path: &Path) -> HashMap<String, Player> {
    if !path.exists() {
        return HashMap::new();
    }
    let mut f = File::open(path).expect("Could not open player file.");
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("Could not read player file to string.");
    serde_json::from_str(&contents).unwrap()
}
