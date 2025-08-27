use crate::api::ApiClient;
use crate::constants::API_BASE_URL;

pub async fn test_api() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating API client...");
    let client = ApiClient::new(API_BASE_URL.to_string())?;
    
    println!("Testing API connection...");
    
    // Now test games/players endpoint
    let endpoint = "/games/players?year=2024&week=1&team=Alabama";
    println!("Making request to: {}", endpoint);
    
    match client.get::<serde_json::Value>(&endpoint).await {
        Ok(response) => {
            println!("Game stats endpoint success! Raw response:");
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Err(e) => {
            println!("Game stats endpoint error: {}", e);
        }
    }
    
    Ok(())
}
