use std::path::{Path, PathBuf};
use std::io::BufReader;
use std::fs::File;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
struct GameTurn {
    dice: [u8; 5],
    rolls: u8,
    score: u8,
}

#[derive(Serialize, Deserialize, Default)]
struct GamePlayer {
    #[serde(default)]
    turns: u64,
    #[serde(default)]
    rolls: u64,

    #[serde(default)]
    total_yahtzees: u64,
    #[serde(default)]
    best_yahtzee_die: u8,

    best_turn: Option<GameTurn>,

    #[serde(skip_serializing)]
    current_turn: Option<GameTurn>,
}

pub struct Yahtzee {
    players: HashMap<String, GamePlayer>,
    path: PathBuf,
}

impl Yahtzee {
    pub fn new(save_path: &Path) -> Self {
        Self {
            players: HashMap::new(),
            path: save_path.to_path_buf(),
        }
    }

    pub fn load_from_file(path: &Path) -> Option<Self> {
        if !path.exists() {
            println!(
                "File {} doesn't exist. Creating new Yahtzee game.",
                path.display()
            );
            return Some(Self::new(path));
        }
        let file = match File::open(path) {
            Ok(f) => f,
            Err(err) => {
                println!("Yahtzee failed to open file {}: {}", path.display(), err);
                return None;
            }
        };

        let reader = BufReader::new(file);

        let players: HashMap<String, GamePlayer> = match serde_json::from_reader(reader) {
            Ok(j) => j,
            Err(err) => {
                println!("Yahtzee failed to parse file {}: {}", path.display(), err);
                return None;
            }
        };

        Some(Self {
            players,
            path: path.to_path_buf(),
        })
    }

    pub fn load_from_default_file() -> Option<Self> {
        Self::load_from_file(&Path::new("yahtzee.json"))
    }

    pub fn save(&self) {
        let file = match File::options().write(true).truncate(true).open(&self.path) {
            Ok(f) => f,
            Err(err) => {
                println!(
                    "Yahtzee failed to write to file {}: {}",
                    self.path.display(),
                    err
                );
                return;
            }
        };

        if let Err(err) = serde_json::to_writer(file, &self.players) {
            println!(
                "Yahtzee failed to serialize for file {}: {}",
                self.path.display(),
                err
            );
        }
    }
}

impl Drop for Yahtzee {
    fn drop(&mut self) {
        self.save()
    }
}
