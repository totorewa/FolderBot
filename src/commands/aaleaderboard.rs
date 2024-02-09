use itertools::Itertools;
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, time::{Duration, SystemTime, UNIX_EPOCH}};

macro_rules! return_if_err {
    ($e:expr) => {
        match $e {
            Err(err) => return err,
            Ok(v) => v,
        }
    };
}

#[derive(Serialize, Deserialize, Debug)]
struct AAPlayer {
    name: String,
    rank: u32,
    #[serde(rename = "runTime")]
    igt: String,
    // #[serde(rename = "dateAccomplished")]
    // date: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct APIResponse {
    players: HashMap<String, AAPlayer>,
    leaderboard: Vec<String>,
    #[serde(rename = "lastUpdated")]
    last_updated: String,
}

#[derive(Default)]
pub struct AALeaderboard {
    data: Option<APIResponse>,
    next_fetch: Option<SystemTime>,
}

impl std::fmt::Display for AAPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = &self.name;
        let igt = &self.igt;
        write!(f, "{} ({})", name, igt)
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
        const URL: &str = "https://totorewa.github.io/aa-leaderboard/1-16-1.json";
        const REFRESH_AFTER: Duration = Duration::from_secs(60 * 60);
        const RETRY_AFTER: Duration = Duration::from_secs(60 * 5);

        let res = match reqwest::get(URL).await {
            Ok(res) => res,
            Err(err) => {
                println!("{}", err);
                return
            }
        };
        match res.json::<APIResponse>().await {
            Ok(lb) => {
                self.data = Some(lb);
                self.next_fetch = Some(SystemTime::now() + REFRESH_AFTER);
            },
            Err(err) => {
                println!("{}", err);
                self.data = None;
                self.next_fetch = Some(SystemTime::now() + RETRY_AFTER);
            }
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.next_fetch.is_some() && self.data.is_some()
    }

    pub fn info_at_rank(&self, rank: u32) -> String {
        if rank < 1 || rank > 100 {
            return "Sorry, I only keep track of the top 100 players.".to_string();
        }

        let lb = return_if_err!(self.get_data());
        lb.leaderboard.get(rank as usize - 1)
            .and_then(|n| AALeaderboard::info_for_normalized_name(lb, n))
            .unwrap_or_else(|| format!("Umm, for some reason I can't find a player at rank {}... folderWoah", rank))
    }

    pub fn info_for_name(&self, name: String) -> String {
        let lb = return_if_err!(self.get_data());
        let normalized_name = name.trim().to_lowercase();
        AALeaderboard::info_for_normalized_name(lb, &normalized_name)
            .unwrap_or_else(|| format!("Sorry, I don't know who {} is. shrujj", name))
    }

    pub fn info_for_streamer(&self) -> String {
        const STREAMER_NAME: &str = "desktopfolder";
        self.info_for_name(STREAMER_NAME.to_string())
    }

    pub fn top_info(&self) -> String {
        let lb = return_if_err!(self.get_data());
        let top5 = lb.leaderboard
            .iter()
            .take(5)
            .map(|p| lb.players
                    .get(p)
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "MISSING???".to_string()))
            .join(" | ");
        format!("Top 5 AA runs: {}", top5)
    }

    pub fn last_update(&self) -> String {
        const GENERIC_ERROR_RESPONSE: &str = "I couldn't figure out when my leaderboard was last updated";
        let lb = return_if_err!(self.get_data());

        let ts = return_if_err!(lb.last_updated.parse::<u64>().map_err(|_| GENERIC_ERROR_RESPONSE.to_string()));

        let time = UNIX_EPOCH + Duration::from_secs(ts);
        let dur_since = return_if_err!(SystemTime::now().duration_since(time).map_err(|_| GENERIC_ERROR_RESPONSE.to_string())).as_secs();
        // super super lazy duration formatting
        let relativity = if dur_since >= 86400 { // a day
                "more than a day ago"
            } else if dur_since >= 43200 { // 12 hours
                "more than 12 hours ago"
            } else if dur_since >= 21600 { // 6 hours
                "more than 6 hours ago"
            } else if dur_since >= 7200 { // 2 hours
                "more than 2 hours ago"
            } else if dur_since >= 3600 { // 1 hour
                "about an hour ago"
            } else if dur_since > 0 {
                "less than an hour ago"
            } else if dur_since == 0 {
                "now, somehow???"
            } else {
                "in the future somehow???"
            };
        format!("My AA leaderboard was last updated {}", relativity)
    }

    fn get_data(&self) -> Result<&APIResponse, String> {
        self.data
            .as_ref()
            .ok_or_else(|| "The AA Leaderboard is not loaded. sajj".to_string())
    }

    fn info_for_normalized_name(data: &APIResponse, normalized_name: &String) -> Option<String> {
        data.players
            .get(normalized_name)
            .map(|p| format!("#{}: {}", p.rank, p))
    }
}

