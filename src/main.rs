mod api;
mod constants;
mod models;
mod fetch_teams;
mod game_stats;
mod test_api;

use api::ApiClient;
use constants::*;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ApiClient::new(API_BASE_URL.to_string())?;

    // --- Default values ---
    let default_team = "Michigan".to_string();
    let default_year = 2024;
    let default_week = 1;
    // --------------------

    let args: Vec<String> = env::args().collect();
    let team = args.get(1).unwrap_or(&default_team);
    let year: u32 = args.get(2).and_then(|y| y.parse().ok()).unwrap_or(default_year);
    let week: u32 = args.get(3).and_then(|w| w.parse().ok()).unwrap_or(default_week);

    game_stats::fetch_and_analyze_qb_stats(&client, team, year, week).await?;
    
    println!("Analysis completed!");
    Ok(())
}
