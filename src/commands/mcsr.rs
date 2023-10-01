use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DefaultOnError;

use std::collections::HashMap;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
struct MCSRRecord {
    #[serde_as(deserialize_as = "DefaultOnError")]
    win: f64,
    #[serde_as(deserialize_as = "DefaultOnError")]
    lose: f64,
    #[serde_as(deserialize_as = "DefaultOnError")]
    draw: f64,
}
// USEFUL SERDE DOCS
// https://serde.rs/enum-representations.html
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
struct MCSRData {
    uuid: String,
    nickname: String,
    //#[serde_as(deserialize_as = "DefaultOnError")]
    //badge: i64,
    elo_rate: i64,
    elo_rank: Option<i64>,
    created_time: i64,
    latest_time: i64,
    total_played: i64,
    season_played: i64,
    highest_winstreak: i64,
    current_winstreak: i64,
    prev_elo_rate: i64,
    best_elo_rate: i64,
    best_record_time: i64,
    records: HashMap<String, MCSRRecord>,
}
#[derive(Serialize, Deserialize, Debug)]
struct MCSRAPIResponse {
    status: String,
    data: MCSRData,
}

impl MCSRData {
    fn win_loss(&self) -> String {
        match self.records.get("2") {
            /*
            Some(MR) => match (MR.win.parse::<f64>(), MR.lose.parse::<f64>()) {
                (Ok(w), Ok(l)) => {
                    format!("[{} - {} ({:.2}%)]", MR.win, MR.lose, w * 100.0 / (l + w))
                }
                _ => format!("[{} - {}]", MR.win, MR.lose),
            },
            */
            Some(mcsr_record) => format!(
                "[{} - {} ({:.2}%)]",
                mcsr_record.win,
                mcsr_record.lose,
                mcsr_record.win * 100.0 / (mcsr_record.lose + mcsr_record.win)
            ),
            None => "[No data]".to_string(),
        }
    }
}
pub async fn lookup(args: String) -> String {
    let un: String = match args.len() {
        0..=2 => "DesktopFolder".into(),
        _ => args.clone(),
    };
    if let Ok(r) = reqwest::get(format!("https://mcsrranked.com/api/users/{}", un)).await {
        match r.json::<MCSRAPIResponse>().await {
            Ok(j) => {
                format!(
                    "Elo for {0}: {1} (Rank #{2}) Season games: {3} {4} Graph -> https://disrespec.tech/elo/?username={0}",
                    un,
                    j.data.elo_rate,
                    j.data.elo_rank.unwrap_or(-1),
                    j.data.season_played,
                    j.data.win_loss(),
                )
            }
            Err(e) => {
                println!("{}", e);
                format!("Bad MCSR API response for {}.", un)
            }
        }
    } else {
        "Failed to query MCSR API.".to_string()
    }
}
