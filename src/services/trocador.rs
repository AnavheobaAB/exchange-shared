use reqwest::Client;

use crate::modules::swap::schema::{TrocadorCurrency, TrocadorProvider};

/// Trocador API client
/// Handles all communication with Trocador.app API
pub struct TrocadorClient {
    client: Client,
    api_key: String,
    base_url: String,
}

#[derive(Debug)]
pub enum TrocadorError {
    HttpError(String),
    ParseError(String),
    ApiError(String),
}

impl std::fmt::Display for TrocadorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrocadorError::HttpError(e) => write!(f, "HTTP error: {}", e),
            TrocadorError::ParseError(e) => write!(f, "Parse error: {}", e),
            TrocadorError::ApiError(e) => write!(f, "API error: {}", e),
        }
    }
}

impl std::error::Error for TrocadorError {}

impl TrocadorClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.trocador.app".to_string(),
        }
    }

    /// Fetch all currencies from Trocador /coins endpoint
    pub async fn get_currencies(&self) -> Result<Vec<TrocadorCurrency>, TrocadorError> {
        let url = format!("{}/coins", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("API-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| TrocadorError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(TrocadorError::ApiError(format!(
                "API returned status: {}",
                response.status()
            )));
        }

        let currencies: Vec<TrocadorCurrency> = response
            .json()
            .await
            .map_err(|e| TrocadorError::ParseError(e.to_string()))?;

        Ok(currencies)
    }

    /// Fetch all providers from Trocador /exchanges endpoint
    pub async fn get_providers(&self) -> Result<Vec<TrocadorProvider>, TrocadorError> {
        let url = format!("{}/exchanges", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("API-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| TrocadorError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(TrocadorError::ApiError(format!(
                "API returned status: {}",
                response.status()
            )));
        }

        // Trocador returns { "list": [...] } not a direct array
        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| TrocadorError::ParseError(e.to_string()))?;

        let providers_array = response_json
            .get("list")
            .ok_or_else(|| TrocadorError::ParseError("Missing 'list' key".to_string()))?;

        let providers: Vec<TrocadorProvider> = serde_json::from_value(providers_array.clone())
            .map_err(|e| TrocadorError::ParseError(e.to_string()))?;

        Ok(providers)
    }
}
