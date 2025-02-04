use base64;
use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use std::fs;

#[derive(Debug, Deserialize)]
struct Config {
    base_url: String,
    client_id: String,
    client_secret: String,
}

#[derive(Debug)]
pub struct RoroApi {
    base_url: String,
    client: Client,
    auth_header: String,
}

#[derive(Debug)]
pub enum ApiError {
    ConfigError(String),
    RequestError(reqwest::Error),
    ApiError(StatusCode),
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        ApiError::RequestError(err)
    }
}

impl RoroApi {
    pub fn new(config_path: &str) -> Result<Self, ApiError> {
        let config = fs::read_to_string(config_path)
            .map_err(|e| ApiError::ConfigError(format!("Failed to read config: {}", e)))?;

        let api_config: Config = serde_json::from_str(&config)
            .map_err(|e| ApiError::ConfigError(format!("Failed to parse config file: {}", e)))?;

        let auth_header = create_basic_auth(&api_config.client_id, &api_config.client_secret);

        Ok(Self {
            base_url: api_config.base_url,
            client: Client::new(),
            auth_header,
        })
    }

    pub fn new_from_default() -> Result<Self, ApiError> {
        Self::new(".roroapi.json")
    }

    pub async fn req_get(
        &self,
        endpoint: &str,
        query_params: Option<&[(&str, &str)]>,
    ) -> Result<Value, ApiError> {
        self.api_request(Method::GET, endpoint, query_params).await
    }

    pub async fn req_post(
        &self,
        endpoint: &str,
        query_params: Option<&[(&str, &str)]>,
    ) -> Result<Value, ApiError> {
        self.api_request(Method::POST, endpoint, query_params).await
    }

    async fn api_request(
        &self,
        method: Method,
        endpoint: &str,
        query_params: Option<&[(&str, &str)]>,
    ) -> Result<Value, ApiError> {
        let url = format!("{}{}", self.base_url, endpoint);

        let mut request = self
            .client
            .request(method, &url)
            .header("Authorization", &self.auth_header);

        if let Some(params) = query_params {
            request = request.query(params);
        }

        let response = request.send().await?;

        let status = response.status();
        let body = response.json::<Value>().await?;

        if status.is_success() {
            Ok(body)
        } else {
            Err(ApiError::ApiError(status))
        }
    }
}

fn create_basic_auth(client_id: &str, client_secret: &str) -> String {
    let credentials = format!("{}:{}", client_id, client_secret);
    let encoded = base64::encode(credentials);
    format!("Basic {}", encoded)
}
