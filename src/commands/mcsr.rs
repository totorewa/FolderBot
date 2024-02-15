use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DefaultOnError;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MCSRTimestamp {
    #[serde_as(deserialize_as = "DefaultOnError")]
    first_online: i64,
    #[serde_as(deserialize_as = "DefaultOnError")]
    last_online: i64,
    #[serde_as(deserialize_as = "DefaultOnError")]
    last_ranked: i64,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
struct MCSRStat {
    ranked: Option<i64>,
    casual: Option<i64>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MCSRStatistics {
    best_time: MCSRStat,
    highest_win_streak: MCSRStat,
    current_win_streak: MCSRStat,
    played_matches: MCSRStat,
    playtime: MCSRStat,
    forfeits: MCSRStat,
    completions: MCSRStat,
    wins: MCSRStat,
    loses: MCSRStat,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
struct MCSRStatData {
    season: MCSRStatistics,
    total: MCSRStatistics,
}

// USEFUL SERDE DOCS
// https://serde.rs/enum-representations.html
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MCSRData {
    uuid: String,
    nickname: String,
    elo_rate: Option<i64>,
    elo_rank: Option<i64>,
    timestamp: MCSRTimestamp,
    statistics: MCSRStatData,
}
#[derive(Serialize, Deserialize, Debug)]
struct MCSRAPIResponse {
    status: String,
    data: MCSRData,
}

impl MCSRStatistics {
    fn win_loss(&self) -> String {
        self.wins
            .ranked
            .and_then(|wins| self.loses.ranked.map(|loses| (wins, loses)))
            .filter(|(w, l)| w + l != 0)
            .map(|(w, l)| {
                format!(
                    "[{} - {} ({:.2}%)]",
                    w,
                    l,
                    (w as f64 * 100.0) / (l as f64 + w as f64)
                )
            })
            .unwrap_or_else(|| "[No data]".to_string())
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
                    "Elo for {0}: {1} (Rank #{2}) Season games: {3} {4} Graph -> https://mcsrranked.com/profile/{5}",
                    un,
                    j.data.elo_rate.map(|r| r.to_string()).unwrap_or("N/A".to_string()),
                    j.data.elo_rank.map(|r| r.to_string()). unwrap_or("N/A".to_string()),
                    j.data.statistics.season.played_matches.ranked.unwrap_or(0),
                    j.data.statistics.season.win_loss(),
                    j.data.nickname,
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
