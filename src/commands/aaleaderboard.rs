use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use itertools::Itertools;

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
const STREAMER_NAME: &str = "desktopfolder";

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

    pub async fn fetch_if_required(&mut self) {
        let now = SystemTime::now();

        if self.data.is_none() || self.next_fetch.map(|t| t <= now).unwrap_or(true) {
            self.fetch().await
        }
    }

    pub async fn fetch(&mut self) {
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

        self.data = dl.fetch().await;
        self.next_fetch = Some(SystemTime::now() + self.data.as_ref().map(|_| REFRESH_AFTER).unwrap_or(RETRY_AFTER));
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
        lb.leaderboard.get(rank as usize - 1)
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
        let top5 = lb.leaderboard
            .iter()
            .take(5)
            .map(|p| p.to_string())
            .join(" | ");
        format!("Top 5 AA runs: {}", top5)
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

    async fn fetch(&mut self) -> Option<AALeaderboardData> {
        self.fetch_csv().await;
        let csv = self.csv.as_ref()?;
        let lines = csv.split('\n').collect_vec();

        let mut rank_idx = 0;
        let mut name_idx = 2;
        let mut igt_idx = 3;
        for (i, cell) in lines.get(1)?.split(',').enumerate() {
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
                    name = cell.clone()
                } else if i == igt_idx {
                    igt = cell.clone()
                }
            }
            let player = Arc::new(AAPlayer { name: name.to_string(), rank, igt: igt.to_string() });
            players.insert(name.trim().to_lowercase(), player.clone());
            leaderboard.push(player.clone());
        }
        
        Some(AALeaderboardData { players, leaderboard })
    }

    async fn fetch_csv(&mut self) -> bool {
        if self.csv.is_some() { return true }
        if let Ok(res) = reqwest::get(&self.url).await {
            self.csv = res.text().await.ok();
            return self.csv.is_some()
        }
        return false
    }

    fn make_export_url(spreadsheet_id: &str, worksheet_id: i64) -> String {
        format!(
            "https://docs.google.com/spreadsheets/d/{}/export?gid={}&format=csv",
            spreadsheet_id, worksheet_id
        )
    }

}