use crate::apis::roroapi::RoroApi;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[derive(Debug)]
struct BoardCacheInfo {
    names: Vec<String>,
    expires_at: SystemTime,
}

#[derive(Debug)]
pub struct LeaderboardClient {
    api_client: RoroApi,
    board_cache: Arc<Mutex<HashMap<String, BoardCacheInfo>>>,
}

#[derive(Debug)]
pub enum LeaderboardError {
    ApiError(String),
    CommandError(String),
}

pub enum LeaderboardGameCategory {
    AnyPercent,
    AllAdvancements,
}

impl LeaderboardClient {
    pub fn new() -> Option<Self> {
        let api_client = RoroApi::new_from_default().ok()?;
        Some(Self {
            api_client,
            board_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn search(&self, category: LeaderboardGameCategory, command: &str) -> Result<String, LeaderboardError> {
        let (board, rest) = self.parse_board(&category, command).await?;
        let mut params = self.parse_command(&rest, &category)?;
        params.insert("cat".to_string(), category.to_string());
        params.insert("board".to_string(), board);

        if let Some(name) = params.get_mut("name") {
            if name.is_empty() {
                *name = "desktopfolder".to_string();
            }
        }

        let params_vec: Vec<(&str, &str)> = params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let response = self
            .api_client
            .req_get("/api/leaderboard/search", Some(&params_vec))
            .await
            .map_err(|_| LeaderboardError::ApiError("Failed to search leaderboard".to_string()))?;

        self.format_response(command, &response, &params)
    }

    async fn parse_board(
        &self,
        category: &LeaderboardGameCategory,
        command: &str,
    ) -> Result<(String, String), LeaderboardError> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(("rsg".to_string(), command.to_string()));
        }

        let boards = self.get_boards(category).await?;
        if boards.contains(&parts[0].to_lowercase()) {
            Ok((parts[0].to_string(), parts[1..].join(" ")))
        } else {
            Ok(("rsg".to_string(), command.to_string()))
        }
    }

    async fn get_boards(&self, category: &LeaderboardGameCategory) -> Result<Vec<String>, LeaderboardError> {
        let mut cache = self.board_cache.lock().unwrap();
        let now = SystemTime::now();

        if let Some(info) = cache.get(&category.to_string()) {
            if now < info.expires_at {
                return Ok(info.names.clone());
            }
        }

        let response = self
            .api_client
            .req_get("/api/leaderboard/boards", Some(&[("cat", category.to_string().as_str())]))
            .await
            .map_err(|_| LeaderboardError::ApiError("Failed to get all boards".to_string()))?;

        let boards: Vec<String> = response
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.get("name").and_then(Value::as_str))
            .map(|s| s.to_lowercase())
            .collect();

        cache.insert(
            category.to_string(),
            BoardCacheInfo {
                names: boards.clone(),
                expires_at: now + Duration::from_secs(3600),
            },
        );

        Ok(boards)
    }

    fn parse_command(
        &self,
        command: &str,
        category: &LeaderboardGameCategory,
    ) -> Result<HashMap<String, String>, LeaderboardError> {
        let mut params = HashMap::new();
        let command = command.trim();

        if command.is_empty() {
            params.insert("name".into(), "desktopfolder".into());
            return Ok(params);
        }

        // Time-based search
        if let Some(_) = command.split_once(':') {
            let (prefix, time) = self.parse_time(command, category)?;
            match prefix {
                Some('<') => params.insert("ltetime".into(), time),
                Some('>') => params.insert("gtetime".into(), time),
                _ => params.insert("gtetime".into(), time),
            };
            return Ok(params);
        }

        // Numerical commands
        let parts: Vec<&str> = command.split_whitespace().collect();
        match parts[0] {
            "top" => self.parse_top(&parts, &mut params),
            "range" => self.parse_range(&parts, &mut params),
            num => self.parse_number(num, &mut params),
        }?;

        Ok(params)
    }

    fn parse_time(
        &self,
        input: &str,
        category: &LeaderboardGameCategory,
    ) -> Result<(Option<char>, String), LeaderboardError> {
        let mut chars = input.chars();
        let first = chars.next().unwrap();

        let (prefix, time_str) = match first {
            '<' | '>' => (Some(first), chars.collect::<String>()),
            _ => (None, input.to_string()),
        };

        let time_parts: Vec<&str> = time_str.split(':').collect();
        let formatted_time = match (category, time_parts.len()) {
            (LeaderboardGameCategory::AllAdvancements, 1) => format!("{}:00:00", time_parts[0].pad_left(2)),
            (LeaderboardGameCategory::AllAdvancements, 2) => format!("{}:00", time_parts.join(":")),
            (LeaderboardGameCategory::AllAdvancements, 3) => time_parts.join(":"),
            (_, 1) => format!("00:00:{}", time_parts[0].pad_left(2)),
            (_, 2) => format!("00:{}", time_parts.join(":")),
            (_, 3) => time_parts.join(":"),
            _ => {
                return Err(LeaderboardError::CommandError(format!(
                    "Invalid time format: {}",
                    input
                )))
            }
        };

        Ok((prefix, formatted_time))
    }

    // Helper functions for command parsing
    fn parse_top(
        &self,
        parts: &[&str],
        params: &mut HashMap<String, String>,
    ) -> Result<(), LeaderboardError> {
        if parts.len() != 2 {
            return Err(LeaderboardError::CommandError("Invalid top command".into()));
        }

        let count: u32 = parts[1]
            .parse()
            .map_err(|_| LeaderboardError::CommandError("Invalid number in top command".into()))?;

        params.insert("place".into(), "1".into());
        params.insert("take".into(), count.to_string());
        Ok(())
    }

    fn parse_range(
        &self,
        parts: &[&str],
        params: &mut HashMap<String, String>,
    ) -> Result<(), LeaderboardError> {
        if parts.len() != 3 {
            return Err(LeaderboardError::CommandError(
                "Invalid range command".into(),
            ));
        }

        let start: u32 = parts[1]
            .parse()
            .map_err(|_| LeaderboardError::CommandError("Invalid start in range".into()))?;
        let end: u32 = parts[2]
            .parse()
            .map_err(|_| LeaderboardError::CommandError("Invalid end in range".into()))?;

        params.insert("place".into(), start.to_string());
        params.insert("take".into(), (end - start + 1).to_string());
        Ok(())
    }

    fn parse_number(
        &self,
        num: &str,
        params: &mut HashMap<String, String>,
    ) -> Result<(), LeaderboardError> {
        match num.parse::<u32>() {
            Ok(n) => {
                params.insert("place".into(), n.to_string());
                params.insert("take".into(), "1".into());
                Ok(())
            }
            Err(_) => {
                params.insert("name".into(), num.to_string());
                Ok(())
            }
        }
    }

    fn format_response(
        &self,
        command: &str,
        response: &Value,
        params: &HashMap<String, String>,
    ) -> Result<String, LeaderboardError> {
        let results = response
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(|| LeaderboardError::ApiError("Invalid response format".into()))?;

        if results.is_empty() {
            return Ok(format!("Sorry, I don't know who {} is. smh", command));
        }

        let is_name_query = params.contains_key("name");
        let max_results = if is_name_query { 1 } else { results.len() };
        let results_to_process = &results[..max_results.min(results.len())];

        let (mut with_time, mut without_time) = (Vec::new(), Vec::new());

        for (idx, result) in results_to_process.iter().enumerate() {
            let run = result
                .get("run")
                .and_then(Value::as_object)
                .ok_or_else(|| LeaderboardError::ApiError("Missing run object".into()))?;

            let players = run
                .get("players")
                .and_then(Value::as_array)
                .ok_or_else(|| LeaderboardError::ApiError("Invalid players format".into()))?;

            let players_str = self.format_players(players)?;
            let time = run
                .get("completionTime")
                .and_then(Value::as_str)
                .unwrap_or("");

            // Only show place for first result in multi-result queries
            let place = if idx == 0 {
                run.get("place").and_then(Value::as_u64).unwrap_or(0)
            } else {
                0
            };

            with_time.push(
                if idx == 0 {
                    format!("#{}: {} ({})", place, players_str, time)
                } else {
                    format!("{} ({})", players_str, time)
                },
            );

            without_time.push(
                if idx == 0 {
                    format!("#{}: {}", place, players_str)
                } else {
                    players_str
                },
            );
        }

        let with_time_str = with_time.join(" | ");
        if with_time_str.len() <= 250 {
            return Ok(with_time_str);
        }

        let without_time_str = without_time.join(" | ");
        if without_time_str.len() <= 250 {
            return Ok(without_time_str);
        }

        Ok("Too many results Smoge".into())
    }

    fn format_players(&self, players: &[Value]) -> Result<String, LeaderboardError> {
        let names: Vec<&str> = players.iter().filter_map(|v| v.as_str()).collect();

        if names.is_empty() {
            return Err(LeaderboardError::ApiError("No players".to_string()));
        }

        Ok(match names.len() {
            1 => names[0].into(),
            2 => format!("{} & {}", names[0], names[1]),
            _ => {
                let mut formatted = names[..names.len() - 1].join(", ");
                formatted.push_str(" & ");
                formatted.push_str(names.last().unwrap());
                formatted
            }
        })
    }
}

impl LeaderboardGameCategory {
    fn to_string(&self) -> String {
        match self {
            LeaderboardGameCategory::AnyPercent => "any%".to_string(),
            LeaderboardGameCategory::AllAdvancements => "aa".to_string(),
        }
    }
}

trait PadLeft {
    fn pad_left(&self, length: usize) -> String;
}

impl PadLeft for str {
    fn pad_left(&self, length: usize) -> String {
        format!("{:0>width$}", self, width = length)
    }
}
