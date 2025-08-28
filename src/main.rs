mod api;
mod constants;
mod models;
mod fetch_teams;
mod game_stats;
mod position_stats;
mod test_api;

use api::ApiClient;
use constants::*;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ApiClient::new(API_BASE_URL.to_string())?;

    // --- Default values ---
    let default_teams = "Texas".to_string();
    let default_year = 2025;
    let default_week = 1;
    // --------------------

    let args: Vec<String> = env::args().collect();
    let teams_input = args.get(1).unwrap_or(&default_teams);
    let year: u32 = args.get(2).and_then(|y| y.parse().ok()).unwrap_or(default_year);
    let week: u32 = args.get(3).and_then(|w| w.parse().ok()).unwrap_or(default_week);

    // Parse teams - can be single team or comma-separated list
    let teams: Vec<&str> = teams_input.split(',').map(|t| t.trim()).collect();
    
    println!("🏈 Starting analysis for {} team(s) in Week {} of {}", teams.len(), week, year);
    println!("Teams: {}", teams.join(", "));
    println!("{}", "=".repeat(60));
    
    // Process each team individually
    for (index, team) in teams.iter().enumerate() {
        println!("\n📊 Processing team {}/{}: {}", index + 1, teams.len(), team);
        println!("{}", "-".repeat(40));
        
        match game_stats::fetch_and_analyze_qb_stats(&client, team, year, week).await {
            Ok(_) => println!("✅ Successfully processed {}", team),
            Err(e) => {
                println!("❌ Error processing {}: {}", team, e);
                // Continue with other teams instead of stopping
                continue;
            }
        }
    }
    
    println!("\n{}", "=".repeat(60));
    println!("🎉 Analysis completed for all {} team(s)!", teams.len());
    println!("📁 Output organized in: output/[team_name]/week_{}/", week);
    Ok(())
}
