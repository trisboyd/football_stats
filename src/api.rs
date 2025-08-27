use crate::constants::*;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::env;
pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Result<Self, Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();
        let api_key = env::var("API_KEY").expect("API_KEY must be set in .env file");
        
        let client = Client::new();

        Ok(ApiClient {
            client,
            base_url,
            api_key,
        })
    }

    // Now async for concurrent requests
    pub async fn get<T: DeserializeOwned>(&self, endpoint: &str) -> Result<T, Box<dyn std::error::Error>> {
        let url = format!("{}{}", self.base_url, endpoint);
        println!("Making request to URL: {}", url);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
            
        println!("Response status: {}", response.status());
        
        let response = response.error_for_status()?;
        let json_response = response.json::<T>().await?;

        Ok(json_response)
    }

    // Convenience methods that just wrap the generic get()
    pub async fn get_teams<T: DeserializeOwned>(&self) -> Result<T, Box<dyn std::error::Error>> {
        self.get(TEAMS_ENDPOINT).await
    }

    pub async fn get_players<T: DeserializeOwned>(&self) -> Result<T, Box<dyn std::error::Error>> {
        self.get(PLAYERS_ENDPOINT).await
    }

    pub async fn get_player_stats_for_game<T: DeserializeOwned>(&self, year: u32, week: u32, team: &str) -> Result<T, Box<dyn std::error::Error>> {
        let endpoint = format!("/stats/{}/week/{}/team/{}", year, week, team);
        self.get(&endpoint).await
    }

    pub async fn get_game_stats<T: DeserializeOwned>(&self, year: u32, week: u32, team: &str) -> Result<T, Box<dyn std::error::Error>> {
        let endpoint = format!("{}?year={}&week={}&team={}", GAME_STATS_ENDPOINT, year, week, team);
        self.get(&endpoint).await
    }
}
