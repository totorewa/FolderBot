use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Player {
    // Basic player metadata
    pub username: String,
    pub nick: Option<String>,
    pub files: i64,        // player's currency
    pub last_message: u64, // last message time THAT ADDED FILES.

    // Game metadata
    pub death: Option<u64>,
    #[serde(default)]
    pub deaths: u64,

    // Tracking metadata :)
    pub sent_messages: u64,
    pub sent_commands: u64,

    // Trident metadata
    pub trident_acc: u64,
    pub max_trident: u64,
    pub tridents_rolled: u64,
    #[serde(default)]
    pub rolled_250s: u32,

    // Enchant metadata
    #[serde(default)]
    pub enchants_rolled: u64,
}

#[derive(Default)]
pub struct PlayerScratch {
    pub last_trident: i32,
    pub greeted: bool,
    pub trident_response_timer: u64,
}

impl PlayerScratch {
    pub fn new() -> PlayerScratch {
        PlayerScratch {
            last_trident: -1,
            greeted: false,
            trident_response_timer: 0,
        }
    }

    pub fn try_dent(&mut self) -> bool {
        let cur = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        if cur > self.trident_response_timer + 1 {
            self.trident_response_timer = cur;
            return true;
        }
        false
    }

    pub fn try_greet(&mut self) -> bool {
        if self.greeted {
            false
        } else {
            self.greeted = true;
            true
        }
    }
}

fn plural<T>(i: T, s: &str) -> String
where
    T: std::fmt::Display + PartialOrd<i64>,
{
    if i == 1 {
        format!("{i} {s}")
    } else {
        format!("{i} {s}s")
    }
}

impl std::fmt::Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        let user = &self.username;
        let files = self.files;
        let coms = self.sent_commands;
        let sent = self.sent_messages - coms;
        let pct = 100.0 * (sent as f64) / (self.sent_messages as f64);
        write!(f, "{} ({}): {}, {}. {} messages sent, {} commands sent ({:.0}%). {:.2} average trident rolled out of {}.",
               name, user, plural(files, "file"), plural(self.deaths as i64, "death"), sent, coms, pct, self.average_trident(), plural(self.tridents_rolled as i64, "roll"))
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
        if self.tridents_rolled == 0 {
            0.0
        } else {
            self.trident_acc as f64 / self.tridents_rolled as f64
        }
    }

    pub fn name(&self) -> String {
        self.nick.clone().unwrap_or(self.username.clone())
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
        self.players
            .entry(name.clone())
            .or_insert_with(|| Player::new(name.clone()))
    }

    pub fn player_or(&mut self, name: &String, other_name: &String) -> &mut Player {
        if self.players.contains_key(name) {
            self.player(name)
        } else {
            self.player(other_name)
        }
    }

    pub fn apply<P>(&mut self, name: &String, predicate: P) -> Option<&Player>
    where
        P: Fn(&mut Player),
    {
        if self.players.contains_key(name) {
            predicate(self.player(name));
            self.players.get(name)
        } else {
            None
        }
    }

    pub fn leaderboard(&self) -> String {
        let itr = self
            .players
            .iter()
            .sorted_by(|a, b| Ord::cmp(&b.1.max_trident, &a.1.max_trident));
        let mut lb = std::vec::Vec::new();
        for (_, d) in itr.take(10) {
            lb.push(format!("{}: {}", d.name(), d.max_trident));
        }
        lb.join(", ")
    }

    pub fn any_leaderboard<P>(&self, predicate: P) -> String
    where
        P: Fn(&Player) -> i64,
    {
        let itr = self
            .players
            .iter()
            .sorted_by(|a, b| Ord::cmp(&predicate(&b.1), &predicate(&a.1)));
        let mut lb = std::vec::Vec::new();
        for (_, d) in itr.take(10) {
            lb.push(format!("{}: {}", d.name(), predicate(&d)));
        }
        lb.join(", ")
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
