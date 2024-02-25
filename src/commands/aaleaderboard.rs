use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use itertools::Itertools;
use lazy_static::lazy_static;
use rand::{thread_rng, Rng};
use reqwest::Client;

macro_rules! return_if_err {
    ($e:expr) => {
        match $e {
            Err(err) => return err,
            Ok(v) => v,
        }
    };
}

const DEFAULT_SPREADSHEET_ID: &str = "107ijqjELTQQ29KW4phUmtvYFTX9-pfHsjb18TKoWACk";
const DEFAULT_WORKSHEET_ID: i64 = 1706556435;
const STREAMER_NAME: &str = "DesktopFolder";

struct AAPlayer {
    name: String,
    rank: u32,
    igt: String,
}

struct AALeaderboardData {
    players: HashMap<String, Arc<AAPlayer>>,
    leaderboard: Vec<Arc<AAPlayer>>,
}

#[derive(Default)]
pub struct AALeaderboard {
    data: Option<AALeaderboardData>,
    next_fetch: Option<SystemTime>,
}

impl std::fmt::Display for AAPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = &self.name;
        let igt = &self.igt;
        write!(f, "{} ({})", name, igt)
    }
}

impl AAPlayer {
    fn to_rank_info(&self) -> String {
        format!("#{}: {}", self.rank, self)
    }
}

impl AALeaderboard {
    pub fn new() -> Self {
        Self { ..Default::default() }
    }

    /// Returns a string if failed to fetch the leaderboard
    pub async fn fetch_if_required(&mut self) -> Option<String> {
        let now = SystemTime::now();

        if self.data.is_none() || self.next_fetch.map(|t| t <= now).unwrap_or(true) {
            return self.fetch().await
        }
        None
    }

    pub async fn fetch(&mut self) -> Option<String> {
        const REFRESH_AFTER: Duration = Duration::from_secs(60 * 60);
        const RETRY_AFTER: Duration = Duration::from_secs(60 * 5);

        let mut dl = LeaderboardDownloader::new(
            &env::var("RAA_LEADERBOARD_SPREADSHEET_ID")
                .unwrap_or_else(|_| DEFAULT_SPREADSHEET_ID.to_string()), 
            env::var("RAA_LEADERBOARD_WORKSHEET_ID")
                .ok()
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(DEFAULT_WORKSHEET_ID),
        );

        match dl.fetch().await {
            Ok(res) => {
                self.data = Some(res);
                self.next_fetch = Some(SystemTime::now() + REFRESH_AFTER);
                None
            },
            Err(err) => {
                self.data = None;
                self.next_fetch = Some(SystemTime::now() + RETRY_AFTER);
                Some(match err {
                    FetchError::Timeout => "Oh no, it's taking too long to read the leaderboard. sajj Am I rate-limited?".to_string(),
                    FetchError::Internal => "If you're seeing this message, then Toto is a bad dev.".to_string(),
                    _ => "I wasn't able to read the leaderboard. sajj".to_string(),
                })
            },
        }
    }

    pub fn unload(&mut self) {
        self.next_fetch = None;
        self.data = None;
    }

    pub fn is_loaded(&self) -> bool {
        self.next_fetch.is_some() && self.data.is_some()
    }

    pub fn info_at_rank(&self, rank: u32) -> String {
        let lb = return_if_err!(self.get_data());
        let idx = return_if_err!((rank as usize).checked_sub(1).ok_or_else(|| "Uhhh, how did we get here?".to_string()));
        lb.leaderboard.get(idx)
            .map(|p| p.to_rank_info())
            .unwrap_or_else(|| format!("I can't find a player at #{}. Hmmge", rank))
    }

    pub fn info_for_name(&self, name: String) -> String {
        let lb = return_if_err!(self.get_data());
        let normalized_name = name.trim().to_lowercase();
        lb.players.get(&normalized_name)
            .map(|p| p.to_rank_info())
            .unwrap_or_else(|| format!("Sorry, I don't know who {} is. shrujj", name))
    }

    pub fn info_for_streamer(&self) -> String {
        self.info_for_name(STREAMER_NAME.to_string())
    }

    pub fn top_info(&self) -> String {
        let lb = return_if_err!(self.get_data());
        let top5 = lb
            .leaderboard
            .iter()
            .take(5)
            .map(if thread_rng().gen_bool(1.0 / 8.0) {
                |p: &Arc<AAPlayer>| format!("{} ({})", &STREAMER_NAME, p.igt)
            } else {
                |p: &Arc<AAPlayer>| p.to_string()
            })
            .join(" | ");
        format!("Top 5 AA runs: {}", top5)
    }

    pub fn best_time(&self) -> String {
        let lb = return_if_err!(self.get_data());
        lb.leaderboard.get(0)
            .map(|p| format!("The best AA time is {} by {}.", p.igt, p.name))
            .unwrap_or_else(|| "I can't find the best time for some reason but it's probably Feinberg. sub 2? probably sub 2.".to_string())
    }

    pub fn slowest_time(&self) -> String {
        let lb = return_if_err!(self.get_data());
        lb.leaderboard.last()
            .map(|p| format!("The slowest time recorded is {}.", p.igt))
            .unwrap_or_else(|| "The slowest time is by the one who hasn't played AA.".to_string())
    }

    fn get_data(&self) -> Result<&AALeaderboardData, String> {
        self.data
            .as_ref()
            .ok_or_else(|| "The AA Leaderboard is not loaded. sajj".to_string())
    }
}

struct LeaderboardDownloader {
    url: String,
    csv: Option<String>,
}

impl LeaderboardDownloader {
    fn new(spreadsheet_id: &String, worksheet_id: i64) -> Self {
        Self {
            url: Self::make_export_url(spreadsheet_id, worksheet_id),
            csv: None,
        }
    }

    async fn fetch(&mut self) -> Result<AALeaderboardData, FetchError> {
        if let Some(err) = self.fetch_csv().await {
            return Err(err)
        }
        let csv = self.csv.as_ref().ok_or(FetchError::General)?;
        let lines = csv.split('\n').collect_vec();

        let mut rank_idx = 0;
        let mut name_idx = 2;
        let mut igt_idx = 3;
        for (i, cell) in lines.get(1).ok_or(FetchError::General)?.split(',').enumerate() {
            match cell {
                "#" => rank_idx = i,
                "Runner" => name_idx = i,
                "IGT" => igt_idx = i,
                _ => continue,
            }
        }

        let mut players = HashMap::<String, Arc<AAPlayer>>::new();
        let mut leaderboard = Vec::<Arc<AAPlayer>>::new();
        for line in lines.iter().skip(2) {
            if line.is_empty() { break }
            let mut rank = 0;
            let mut name = "";
            let mut igt = "";
            for (i, cell) in line.split(',').enumerate() {
                if i == rank_idx {
                    rank = cell.parse::<u32>().unwrap_or(rank)
                } else if i == name_idx {
                    name = cell
                } else if i == igt_idx {
                    igt = cell
                }
            }
            let player = Arc::new(AAPlayer { name: name.to_string(), rank, igt: igt.to_string() });
            players.insert(name.trim().to_lowercase(), player.clone());
            leaderboard.push(player.clone());
        }
        
        Ok(AALeaderboardData { players, leaderboard })
    }

    async fn fetch_csv(&mut self) -> Option<FetchError> {
        lazy_static! {
            static ref CLIENT: Option<Client> = Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .ok();
        }
        if CLIENT.is_none() { return Some(FetchError::Internal) }
        if self.csv.is_some() { return None }
        match CLIENT.as_ref().unwrap().get(&self.url).send().await {
            Ok(res) => {
                self.csv = res.text().await.ok();
                if self.csv.is_some() { None }
                else { Some(FetchError::General) }
            },
            Err(err) => {
                if err.is_timeout() { Some(FetchError::Timeout) }
                else { Some(FetchError::General) }
            }
        }
    }

    fn make_export_url(spreadsheet_id: &str, worksheet_id: i64) -> String {
        format!(
            "https://docs.google.com/spreadsheets/d/{}/export?gid={}&format=csv",
            spreadsheet_id, worksheet_id
        )
    }

}

enum FetchError {
    General,
    Timeout,
    Internal,
}
