use std::cmp::max;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use itertools::Itertools;
use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

pub const DICE_COUNT: usize = 5;

#[derive(Clone, Copy, Serialize, Deserialize, Default)]
struct GameTurn {
    dice: [u8; DICE_COUNT],
    rolls: u8,
    score: u8,
    #[serde(default, skip_serializing)]
    last_rolled_at: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct GamePlayer {
    #[serde(default)]
    turns: u64,
    #[serde(default)]
    rolls: u64,

    #[serde(default)]
    total_score: u64,

    #[serde(default)]
    total_yahtzees: u64,
    #[serde(default)]
    best_yahtzee_die: u8,

    best_turn: Option<GameTurn>,
    current_turn: Option<GameTurn>,
}

#[derive(Serialize, Deserialize)]
pub struct Yahtzee {
    players: HashMap<String, GamePlayer>,
    cooldown: Option<u64>,

    #[serde(default, skip_serializing)]
    last_roll: u64,

    #[serde(default, skip_serializing)]
    path: PathBuf,
}

impl GameTurn {
    const MAX_ROLLS: u8 = 3;
    const YAHTZEE_SCORE: u8 = 50;

    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn roll(&mut self, saves: &[u8], rng: &mut ThreadRng) -> Result<(), YahtzeeError> {
        if self.rolls >= Self::MAX_ROLLS {
            return Err(YahtzeeError::public(
                "Erm you've already re-rolled twice, {ur} smh",
            ));
        }
        let mut saved_rolls = [0u8; 6];
        for s in saves {
            if let Some(face) = saved_rolls.get_mut(*s as usize - 1) {
                *face += 1;
            }
        }
        let mut rolls = self.dice.clone();
        for die in rolls.iter_mut() {
            if *die != 0 {
                if let Some(face) = saved_rolls.get_mut(*die as usize - 1).filter(|f| **f != 0) {
                    *face -= 1;
                    continue;
                }
            }
            *die = rng.gen_range(1..=6);
        }
        if let Some((face, _)) = saved_rolls.iter().enumerate().find(|s| *s.1 != 0) {
            return Err(YahtzeeError::public(&format!(
                "Hmmge you don't have enough {} dice, {{ur}}",
                get_dice_face_text(face as u8 + 1)
            )));
        }
        self.rolls += 1;
        self.dice = rolls;
        self.last_rolled_at = get_unixtime();
        Ok(())
    }

    /// Calculates best score in-place
    fn calculate_score(&mut self) -> Result<(), YahtzeeError> {
        self.score = 0; // Reset score prior to calculation

        // this algorithm is so bad
        // what is going on rn send help
        // can this be done in O(n)?
        let mut sum = 0u8;
        let mut faces = [0u8; 6];
        let mut house_skip = false;
        let mut straight_skip = false;
        for die in self.dice {
            let face = faces.get_mut(die as usize - 1).ok_or_else(|| {
                YahtzeeError::private(&format!("Turn has an illegal dice roll of {}", die))
            })?;
            *face += 1;
            if *face == DICE_COUNT as u8 {
                // YAHTZEE - early exit
                self.score = Self::YAHTZEE_SCORE;
                return Ok(());
            }
            if !house_skip && *face > 3 {
                // Full house is impossible, skip
                house_skip = true;
            }
            if !straight_skip && *face > 2 {
                // Small and large straight is impossible, skip
                straight_skip = true;
            }
            sum += die;
        }

        // Check for small or large straight
        if !straight_skip {
            let mut max_seq = 0u8;
            let mut seq = 0u8;
            for face in faces {
                if face != 0 {
                    seq += 1;
                } else {
                    if seq > max_seq {
                        max_seq = seq;
                    }
                    seq = 0;
                }
            }
            self.score = match max(max_seq, seq) {
                // LARGE STRAIGHT
                5 => 40,
                // SMALL STRAIGHT
                4 => 30,
                _ => 0,
            };
            if self.score != 0 {
                return Ok(());
            }
        }

        // Check for full house
        // If sum is already 25 or over, don't bother checking for full house
        if !house_skip && sum < 25 {
            let mut kinds = 0u8;
            for face in faces {
                // full house is impossible if there is 1 of a kind
                if face == 1 {
                    break;
                }
                // the full house check is skipped if there is 4 or 5 of a kind so if face is not zero, it must be either 2 or 3
                if face != 0 {
                    if kinds == 0 {
                        kinds = face;
                        continue;
                    }
                    // FULL HOUSE
                    // If kinds and face are equal then there must've been two instances of two of a kind
                    if kinds != face {
                        self.score = 25;
                        return Ok(());
                    }
                }
            }
        }

        self.score = sum;
        Ok(())
    }
}

impl GamePlayer {
    fn play(
        &mut self,
        saves: &[u8],
        cooldown: u64,
        rng: &mut ThreadRng,
    ) -> Result<([u8; 5], u8), YahtzeeError> {
        if let Some(turn) = self.current_turn.as_ref() {
            if get_unixtime() - turn.last_rolled_at < cooldown {
                return Err(YahtzeeError::private(
                    &"Could not start new turn because player cooldown is active",
                ));
            }
            if saves.len() != 0 && turn.rolls >= GameTurn::MAX_ROLLS {
                return Err(YahtzeeError::public(
                    "Erm you've already re-rolled twice, {ur} smh Start a fresh roll with !yahtzee",
                ));
            }
        } else if saves.len() != 0 {
            return Err(YahtzeeError::public(
                "Erm you can't re-roll, {ur} - you haven't rolled yet!",
            ));
        }
        if saves.len() == 0 {
            self.end_turn();
            self.current_turn = Some(GameTurn::new());
        }
        if let Some(turn) = self.current_turn.as_mut() {
            let _ = turn.roll(saves, rng)?;
            turn.calculate_score()
                .map(|_| (turn.dice.to_owned(), turn.score))
        } else {
            Err(YahtzeeError::private(
                "Somehow reached unreachable point in Yahtzee code",
            ))
        }
    }

    fn end_turn(&mut self) {
        if let Some(turn) = self.current_turn.as_mut() {
            if self.best_turn.filter(|bt| bt.score >= turn.score).is_none() {
                self.best_turn = Some(turn.to_owned());
            }
            self.turns += 1;
            self.rolls += turn.rolls as u64;
            self.total_score += turn.score as u64;
            if turn.score == GameTurn::YAHTZEE_SCORE {
                self.total_yahtzees += 1;
                self.best_yahtzee_die = max(turn.dice[0], self.best_yahtzee_die);
            }
        }
        self.current_turn = None;
    }

    fn total_rolls(&self) -> u64 {
        self.rolls
            + self
                .current_turn
                .map(|t| t.rolls as u64)
                .unwrap_or_default()
    }

    fn total_turns(&self) -> u64 {
        self.turns + self.current_turn.map(|_| 1u64).unwrap_or_default()
    }

    fn total_score(&self) -> u64 {
        self.total_score
            + self
                .current_turn
                .map(|t| t.score as u64)
                .unwrap_or_default()
    }

    fn total_yahtzees(&self) -> u64 {
        self.total_yahtzees
            + self
                .current_turn
                .filter(|t| t.score == GameTurn::YAHTZEE_SCORE)
                .map(|_| 1u64)
                .unwrap_or_default()
    }

    fn best_score(&self) -> Option<u8> {
        [
            self.best_turn.as_ref().map(|t| t.score),
            self.current_turn.as_ref().map(|t| t.score),
        ]
        .iter()
        .filter(|s| s.is_some())
        .map(|s| s.unwrap())
        .max()
    }
}

impl Yahtzee {
    pub fn new(save_path: &Path) -> Self {
        Self {
            players: HashMap::new(),
            path: save_path.to_path_buf(),
            last_roll: 0,
            cooldown: None,
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

        let mut opt: Option<Yahtzee> = Some(match serde_json::from_reader(reader) {
            Ok(j) => j,
            Err(err) => {
                println!("Yahtzee failed to parse file {}: {}", path.display(), err);
                return None;
            }
        });

        if let Some(game) = opt.as_mut() {
            game.end_all_turns();
            game.path = path.to_path_buf();
        }
        opt
    }

    pub fn load_from_default_file() -> Option<Self> {
        Self::load_from_file(&Path::new("yahtzee.json"))
    }

    pub fn save(&self) {
        if !self.path.exists() {
            let _ = File::create(&self.path);
        }
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

        if let Err(err) = serde_json::to_writer_pretty(file, self) {
            println!(
                "Yahtzee failed to serialize for file {}: {}",
                self.path.display(),
                err
            );
        }
    }

    pub fn play(&mut self, player_name: &str, saves: &[u8]) -> Result<String, YahtzeeError> {
        self.last_roll = get_unixtime();
        let cd = self.cooldown.unwrap_or(2000);
        let player = self.get_or_create_player(player_name);
        let mut rng = thread_rng();

        let disposed_score = player
            .current_turn
            .as_ref()
            .map(|t| t.score)
            .unwrap_or_default();
        let is_rerolling_yahtzee = saves.len() > 0 && disposed_score == GameTurn::YAHTZEE_SCORE;

        let (rolls, score) = player.play(saves, cd, &mut rng)?;
        let roll_txt = rolls.iter().map(|v| get_dice_face_text(*v)).join(", ");
        if player.best_turn.is_none() && player.current_turn.map(|t| t.rolls).unwrap_or(1) == 1 {
            if score == GameTurn::YAHTZEE_SCORE {
                return Ok(format!("{{ur}} rolled {} and scored YAHTZEE on their first roll!! folderWoah I would've taught them how to re-roll but I don't recommend it.", roll_txt));
            }
            Ok(format!("{{ur}} rolled {} which scores {}. You can re-roll some dice but if you do you won't keep this score! Specify which dice values you wish to keep: e.g. \"!yahtzee {} {} {}\" otherwise roll all dice again to keep your score.", roll_txt, score, rolls[0], rolls[2], rolls[3]))
        } else if score == GameTurn::YAHTZEE_SCORE {
            if is_rerolling_yahtzee {
                Ok(format!("monkaS {{ur}} just threw away their Yahtzee... FOR ANOTHER YAHTZEE! IMDEAD {{ur}} rolled {}", roll_txt))
            } else {
                Ok(format!("YAHTZEE! {{ur}} rolled {} PagMan", roll_txt))
            }
        } else if is_rerolling_yahtzee {
            Ok(format!(
                "{{ur}} rolled {} for {} points... wait, did they just re-roll their yahtzee? WHAT",
                roll_txt, score
            ))
        } else if disposed_score != 0 && saves.len() > 0 {
            if disposed_score == score {
                Ok(format!(
                    "{{ur}} re-rolled {} for {} points... no change in score",
                    disposed_score, roll_txt
                ))
            } else if disposed_score < score {
                Ok(format!(
                    "{{ur}} re-rolled {} for {} points, gaining {} point(s)",
                    roll_txt,
                    score,
                    score - disposed_score
                ))
            } else {
                Ok(format!(
                    "{{ur}} re-rolled {} for {} points, losing {} point(s) oof",
                    roll_txt,
                    score,
                    disposed_score - score
                ))
            }
        } else {
            Ok(format!("{{ur}} rolled {} for {} points", roll_txt, score))
        }
    }

    pub fn end_turn(&mut self, player_name: &str) {
        let player = self.get_or_create_player(player_name);
        player.end_turn()
    }

    pub fn end_all_turns(&mut self) {
        for p in self.players.values_mut() {
            p.end_turn()
        }
    }

    pub fn player_stats(&self, player_name: &str) -> String {
        let normalized_name = player_name.to_lowercase();
        let player = match self.players.get(&normalized_name) {
            Some(p) => p,
            None => {
                return format!("I don't see a player named {}... folderSus", player_name);
            }
        };
        let rolls = player.total_rolls();
        let turns = player.total_turns();
        let total_score = player.total_score();
        let avg_score = if turns == 0 {
            0_f64
        } else {
            total_score as f64 / turns as f64
        };
        format!("{} has rolled {} time(s) with {} re-roll(s). Total score: {}. Average score: {:.2}. Best score: {}. Yahtzee(s): {}.", player_name, turns, rolls as i64 - turns as i64, total_score, avg_score, player.best_score().unwrap_or_default(), player.total_yahtzees())
    }

    pub fn get_total_yahtzees(&self, player_name: &str) -> u64 {
        self.players
            .get(&player_name.to_lowercase())
            .map(|p| p.total_yahtzees())
            .unwrap_or_default()
    }

    fn get_or_create_player(&mut self, player_name: &str) -> &mut GamePlayer {
        let player_name = player_name.to_lowercase();
        self.players.entry(player_name).or_insert(GamePlayer {
            ..Default::default()
        })
    }
}

impl Drop for Yahtzee {
    fn drop(&mut self) {
        self.save()
    }
}

pub enum YahtzeeError {
    Private(String),
    Public(String),
}

impl YahtzeeError {
    fn private(reason: &str) -> Self {
        Self::Private(reason.to_string())
    }

    fn public(display: &str) -> Self {
        Self::Public(display.to_string())
    }
}

impl std::fmt::Display for YahtzeeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                YahtzeeError::Private(reason) => reason,
                YahtzeeError::Public(display) => display,
            }
        )
    }
}

pub fn get_dice_face_text(face: u8) -> &'static str {
    match face {
        1 => "[1]",
        2 => "[2]",
        3 => "[3]",
        4 => "[4]",
        5 => "[5]",
        6 => "[6]",
        _ => "[?]",
        // 1 => "\u{0031}\u{FE0F}\u{20E3}",
        // 2 => "\u{0032}\u{FE0F}\u{20E3}",
        // 3 => "\u{0033}\u{FE0F}\u{20E3}",
        // 4 => "\u{0034}\u{FE0F}\u{20E3}",
        // 5 => "\u{0035}\u{FE0F}\u{20E3}",
        // 6 => "\u{0036}\u{FE0F}\u{20E3}",
        // _ => "\u{0023}\u{FE0F}\u{20E3}",
    }
}

fn get_unixtime() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::GameTurn;

    #[test]
    fn full_house() {
        let mut turn = GameTurn {
            dice: [1, 1, 2, 1, 2],
            ..Default::default()
        };
        let res = turn.calculate_score();
        assert!(
            res.is_ok(),
            "Calculate score returned error: {}",
            res.unwrap_err()
        );
        assert_eq!(
            25, turn.score,
            "Expected full house (25) but got {}",
            turn.score
        );
    }

    #[test]
    fn small_straight() {
        let mut turn = GameTurn {
            dice: [4, 1, 2, 3, 6],
            ..Default::default()
        };
        let res = turn.calculate_score();
        assert!(
            res.is_ok(),
            "Calculate score returned error: {}",
            res.unwrap_err()
        );
        assert_eq!(
            30, turn.score,
            "Expected small straight (30) but got {}",
            turn.score
        );
    }

    #[test]
    fn small_straight_with_duplicate() {
        let mut turn = GameTurn {
            dice: [2, 5, 4, 3, 2],
            ..Default::default()
        };
        let res = turn.calculate_score();
        assert!(
            res.is_ok(),
            "Calculate score returned error: {}",
            res.unwrap_err()
        );
        assert_eq!(
            30, turn.score,
            "Expected small straight (30) but got {}",
            turn.score
        );
    }

    #[test]
    fn large_straight() {
        let mut turn = GameTurn {
            dice: [4, 5, 2, 1, 3],
            ..Default::default()
        };
        let res = turn.calculate_score();
        assert!(
            res.is_ok(),
            "Calculate score returned error: {}",
            res.unwrap_err()
        );
        assert_eq!(
            40, turn.score,
            "Expected large straight (40) but got {}",
            turn.score
        );
    }

    #[test]
    fn yahtzee() {
        let mut turn = GameTurn {
            dice: [4, 4, 4, 4, 4],
            ..Default::default()
        };
        let res = turn.calculate_score();
        assert!(
            res.is_ok(),
            "Calculate score returned error: {}",
            res.unwrap_err()
        );
        assert_eq!(
            50, turn.score,
            "Expected large straight (50) but got {}",
            turn.score
        );
    }
}
