use crate::api::ApiClient;
use crate::constants::*;
use crate::models::*;
use csv::Writer;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub async fn fetch_and_analyze_qb_stats(
    client: &ApiClient,
    team: &str,
    year: u32,
    week: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching game stats for {} in week {} of {}...", team, week, year);
    
    let encoded_team = team.replace(" ", "%20");
    let endpoint = format!("{}?year={}&week={}&team={}", 
        GAME_STATS_ENDPOINT, year, week, encoded_team);
    
    println!("Making API request to endpoint: {}", endpoint);
    
    let games: Vec<Game> = client.get(&endpoint).await?;
    
    if games.is_empty() {
        println!("No game data found for {} in week {} of {}", team, week, year);
        return Ok(());
    }
    
    // Assuming the first game is the correct one for the given team, year, and week
    let game = &games[0];
    let team_names: Vec<String> = game.teams.iter().map(|t| t.school.clone()).collect();
    println!("Found game with teams: {}", team_names.join(" vs "));
    
    process_game_response(&game.teams, team, year, week).await?;
    
    Ok(())
}

async fn process_game_response(
    teams: &[ApiTeamStats],
    target_team: &str,
    year: u32,
    week: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Find opponent name for display purposes
    let opponent_name = teams.iter()
        .find(|team_stats| !team_stats.school.to_lowercase().contains(&target_team.to_lowercase()))
        .map(|stats| stats.school.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    
    println!("Processing stats for {} vs {}", target_team, opponent_name);
    
    // Use new method that processes sack data
    let qb_stats_list = extract_qb_stats_with_sack_data(teams, target_team, year, week).await;
    
    if qb_stats_list.is_empty() {
        println!("No QB stats found for {}", target_team);
    } else {
        println!("Found {} QB records", qb_stats_list.len());
    }
    
    write_qb_stats_to_csv(&qb_stats_list, target_team, week, &opponent_name)?;
    
    Ok(())
}

fn extract_qb_stats_from_api_data(
    team_stats: &ApiTeamStats, 
    opponent: &str, 
    week: u32
) -> Vec<QBStats> {
    let mut qb_data: HashMap<String, QBStatsBuilder> = HashMap::new();
    
    // Process each stat category
    for stat_category in &team_stats.stats {
        match stat_category.name.as_str() {
            "passing" => process_passing_stats(&stat_category.types, &mut qb_data),
            "rushing" => process_rushing_stats(&stat_category.types, &mut qb_data),
            _ => {} // Ignore other categories for now
        }
    }
    
    // Convert builders to final QB stats
    qb_data.into_iter()
        .filter_map(|(player_name, builder)| {
            builder.build(&player_name, &team_stats.school, opponent, week)
        })
        .collect()
}

async fn extract_qb_stats_with_sack_data(
    teams: &[ApiTeamStats],
    target_team: &str,
    year: u32,
    week: u32
) -> Vec<QBStats> {
    let mut target_team_stats = None;
    let mut opponent_name = String::new();
    
    // Find target team stats
    for team_stats in teams {
        if team_stats.school.to_lowercase().contains(&target_team.to_lowercase()) {
            target_team_stats = Some(team_stats);
        } else {
            opponent_name = team_stats.school.clone();
        }
    }
    
    let target_stats = match target_team_stats {
        Some(stats) => stats,
        None => return Vec::new(),
    };
    
    // Get exact sack data from plays API
    let qb_sacks = match get_qb_sacks_from_plays_api(&target_stats.school, year, week).await {
        Ok(sacks) => sacks,
        Err(e) => {
            println!("Warning: Could not get sack data from plays API: {}", e);
            HashMap::new()
        }
    };
    
    let mut qb_data: HashMap<String, QBStatsBuilder> = HashMap::new();
    
    // Process each stat category for target team
    for stat_category in &target_stats.stats {
        match stat_category.name.as_str() {
            "passing" => process_passing_stats(&stat_category.types, &mut qb_data),
            "rushing" => process_rushing_stats(&stat_category.types, &mut qb_data),
            _ => {} // Ignore other categories for now
        }
    }
    
    // Convert builders to final QB stats with exact sack data
    qb_data.into_iter()
        .filter_map(|(player_name, builder)| {
            let (sacks, yards_lost_from_sacks) = qb_sacks.get(&player_name)
                .map(|(sack_count, sack_yards)| (*sack_count as i32, *sack_yards))
                .unwrap_or((0, 0));
                
            builder.build_with_exact_sacks(&player_name, &target_stats.school, &opponent_name, week, sacks, yards_lost_from_sacks)
        })
        .collect()
}

#[derive(Default, Debug)]
struct SackData {
    total_sacks: i32,
    // For now, we'll estimate sack yardage. In future, we could try to get this from play-by-play data
    estimated_sack_yards: i32,
}

fn extract_sack_data(opponent_stats: &ApiTeamStats) -> SackData {
    let mut sack_data = SackData::default();
    
    for stat_category in &opponent_stats.stats {
        if stat_category.name == "defensive" {
            for stat_type in &stat_category.types {
                if stat_type.name == "SACKS" {
                    // Sum up all sacks by defensive players
                    for athlete in &stat_type.athletes {
                        if let Ok(sacks) = athlete.stat.parse::<i32>() {
                            sack_data.total_sacks += sacks;
                        }
                    }
                    
                    // Estimate sack yardage (typical sack loses 7 yards in college football)
                    sack_data.estimated_sack_yards = sack_data.total_sacks * 7;
                    break;
                }
            }
            break;
        }
    }
    
    println!("Extracted sack data: {} sacks, estimated {} yards lost", 
        sack_data.total_sacks, sack_data.estimated_sack_yards);
    
    sack_data
}

#[derive(Default)]
struct QBStatsBuilder {
    passing_attempts: i32,
    completions: i32,
    passing_yards: i32,
    passing_tds: i32,
    interceptions: i32,
    rushing_attempts: i32,
    rushing_yards: i32,
    qbr: f64,
}

impl QBStatsBuilder {
    fn build(self, player: &str, team: &str, opponent: &str, week: u32) -> Option<QBStats> {
        // Only include players who have passing attempts (QBs)
        if self.passing_attempts == 0 {
            return None;
        }
        
        // Calculate derived stats
        let yards_per_attempt = if self.passing_attempts > 0 {
            self.passing_yards as f64 / self.passing_attempts as f64
        } else {
            0.0
        };
        
        let int_rate = if self.passing_attempts > 0 {
            (self.interceptions as f64 / self.passing_attempts as f64) * 100.0
        } else {
            0.0
        };
        
        // For now, we don't have sack data directly, so we'll set these to 0
        // In a more complete implementation, we'd look for defensive stats against this team
        let sacks = 0;
        let sack_rate = 0.0;
        let yards_lost_from_sacks = 0;
        
        // True rushing yards (in college, sack yards are deducted from rushing)
        let true_rushing_yards = self.rushing_yards; // We'd add back sack yards if we had them
        
        Some(QBStats {
            player: player.to_string(),
            team: team.to_string(),
            opponent: opponent.to_string(),
            week,
            passing_attempts: self.passing_attempts,
            completions: self.completions,
            passing_yards: self.passing_yards,
            yards_per_attempt,
            passing_tds: self.passing_tds,
            interceptions: self.interceptions,
            int_rate,
            sacks,
            sack_rate,
            true_rushing_yards,
            yards_lost_from_sacks,
        })
    }

    fn build_with_sacks(self, player: &str, team: &str, opponent: &str, week: u32, sack_data: &SackData) -> Option<QBStats> {
        // Only include players who have passing attempts (QBs)
        if self.passing_attempts == 0 {
            return None;
        }
        
        // Calculate derived stats
        let yards_per_attempt = if self.passing_attempts > 0 {
            self.passing_yards as f64 / self.passing_attempts as f64
        } else {
            0.0
        };
        
        let int_rate = if self.passing_attempts > 0 {
            (self.interceptions as f64 / self.passing_attempts as f64) * 100.0
        } else {
            0.0
        };
        
        // Estimate QB's share of team sacks based on passing attempts
        // This is a rough estimate - in reality, we'd want play-by-play data to know exactly which QB was sacked
        let total_dropbacks = self.passing_attempts + self.rushing_attempts;
        let estimated_qb_sacks = if total_dropbacks > 0 {
            // Estimate this QB's share of the sacks based on their dropback proportion
            // This is imperfect but better than assuming 0
            let qb_share = self.passing_attempts as f64 / (self.passing_attempts as f64 + 10.0); // Assume some baseline
            (sack_data.total_sacks as f64 * qb_share).round() as i32
        } else {
            0
        };
        
        let estimated_qb_sack_yards = if total_dropbacks > 0 {
            let qb_share = self.passing_attempts as f64 / (self.passing_attempts as f64 + 10.0);
            (sack_data.estimated_sack_yards as f64 * qb_share).round() as i32
        } else {
            0
        };
        
        // Calculate sack rate (sacks per passing attempt)
        let sack_rate = if self.passing_attempts > 0 {
            (estimated_qb_sacks as f64 / self.passing_attempts as f64) * 100.0
        } else {
            0.0
        };
        
        // True rushing yards: add back estimated sack yardage to get actual running yardage
        // In college, sacks count as negative rushing yards
        let true_rushing_yards = self.rushing_yards + estimated_qb_sack_yards;
        
        println!("QB {}: rushing_yards={}, estimated_sack_yards={}, true_rushing_yards={}", 
            player, self.rushing_yards, estimated_qb_sack_yards, true_rushing_yards);
        
        Some(QBStats {
            player: player.to_string(),
            team: team.to_string(),
            opponent: opponent.to_string(),
            week,
            passing_attempts: self.passing_attempts,
            completions: self.completions,
            passing_yards: self.passing_yards,
            yards_per_attempt,
            passing_tds: self.passing_tds,
            interceptions: self.interceptions,
            int_rate,
            sacks: estimated_qb_sacks,
            sack_rate,
            true_rushing_yards,
            yards_lost_from_sacks: estimated_qb_sack_yards,
        })
    }

    fn build_with_exact_sacks(self, player: &str, team: &str, opponent: &str, week: u32, sacks: i32, yards_lost_from_sacks: i32) -> Option<QBStats> {
        // Only include players who have passing attempts (QBs)
        if self.passing_attempts == 0 {
            return None;
        }
        
        // Calculate derived stats
        let yards_per_attempt = if self.passing_attempts > 0 {
            self.passing_yards as f64 / self.passing_attempts as f64
        } else {
            0.0
        };
        
        let int_rate = if self.passing_attempts > 0 {
            (self.interceptions as f64 / self.passing_attempts as f64) * 100.0
        } else {
            0.0
        };
        
        // Calculate sack rate (sacks per passing attempt)
        let sack_rate = if self.passing_attempts > 0 {
            (sacks as f64 / self.passing_attempts as f64) * 100.0
        } else {
            0.0
        };
        
        // True rushing yards: subtract sack yardage from rushing stats to get actual running yardage
        // In college, sacks count as negative rushing yards, so we need to remove that impact
        let true_rushing_yards = self.rushing_yards - yards_lost_from_sacks;
        
        println!("QB {}: rushing_yards={}, sack_yards={}, true_rushing_yards={}", 
            player, self.rushing_yards, yards_lost_from_sacks, true_rushing_yards);
        
        Some(QBStats {
            player: player.to_string(),
            team: team.to_string(),
            opponent: opponent.to_string(),
            week,
            passing_attempts: self.passing_attempts,
            completions: self.completions,
            passing_yards: self.passing_yards,
            yards_per_attempt,
            passing_tds: self.passing_tds,
            interceptions: self.interceptions,
            int_rate,
            sacks,
            sack_rate,
            true_rushing_yards,
            yards_lost_from_sacks: -yards_lost_from_sacks, // Make positive for display (yards lost)
        })
    }
}

fn process_passing_stats(stat_types: &[ApiStatType], qb_data: &mut HashMap<String, QBStatsBuilder>) {
    for stat_type in stat_types {
        match stat_type.name.as_str() {
            "C/ATT" => {
                for athlete in &stat_type.athletes {
                    let builder = qb_data.entry(athlete.name.clone()).or_default();
                    if let Some((completions, attempts)) = parse_fraction(&athlete.stat) {
                        builder.completions = completions;
                        builder.passing_attempts = attempts;
                    }
                }
            },
            "YDS" => {
                for athlete in &stat_type.athletes {
                    let builder = qb_data.entry(athlete.name.clone()).or_default();
                    builder.passing_yards = athlete.stat.parse().unwrap_or(0);
                }
            },
            "TD" => {
                for athlete in &stat_type.athletes {
                    let builder = qb_data.entry(athlete.name.clone()).or_default();
                    builder.passing_tds = athlete.stat.parse().unwrap_or(0);
                }
            },
            "INT" => {
                for athlete in &stat_type.athletes {
                    let builder = qb_data.entry(athlete.name.clone()).or_default();
                    builder.interceptions = athlete.stat.parse().unwrap_or(0);
                }
            },
            "QBR" => {
                for athlete in &stat_type.athletes {
                    let builder = qb_data.entry(athlete.name.clone()).or_default();
                    builder.qbr = athlete.stat.parse().unwrap_or(0.0);
                }
            },
            _ => {}
        }
    }
}

fn process_rushing_stats(stat_types: &[ApiStatType], qb_data: &mut HashMap<String, QBStatsBuilder>) {
    for stat_type in stat_types {
        match stat_type.name.as_str() {
            "ATT" => {
                for athlete in &stat_type.athletes {
                    // Only add rushing attempts for players we already know are QBs
                    if let Some(builder) = qb_data.get_mut(&athlete.name) {
                        builder.rushing_attempts = athlete.stat.parse().unwrap_or(0);
                    }
                }
            },
            "YDS" => {
                for athlete in &stat_type.athletes {
                    // Only add rushing yards for players we already know are QBs
                    if let Some(builder) = qb_data.get_mut(&athlete.name) {
                        builder.rushing_yards = athlete.stat.parse().unwrap_or(0);
                    }
                }
            },
            _ => {}
        }
    }
}

fn parse_fraction(s: &str) -> Option<(i32, i32)> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 2 {
        let numerator = parts[0].parse().ok()?;
        let denominator = parts[1].parse().ok()?;
        Some((numerator, denominator))
    } else {
        None
    }
}

fn write_qb_stats_to_csv(
    qb_stats: &[QBStats],
    team: &str,
    week: u32,
    opponent: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure output directory exists
    fs::create_dir_all(OUTPUT_DIR)?;
    
    // Create filename
    let safe_team = team.replace(' ', "_").replace("&", "and");
    let safe_opponent = opponent.replace(' ', "_").replace("&", "and");
    let filename = format!("{}/QB_Stats_{}_vs_{}_Week_{}.csv", 
        OUTPUT_DIR, safe_team, safe_opponent, week);
    let path = Path::new(&filename);
    
    let mut writer = Writer::from_path(path)?;
    
    // Write headers
    writer.write_record(&[
        "Player",
        "Team", 
        "Opponent",
        "Week",
        "Passing_Attempts",
        "Completions",
        "Passing_Yards",
        "Yards_Per_Attempt",
        "Passing_TDs",
        "Interceptions", 
        "INT_Rate_%",
        "Sacks",
        "Sack_Rate_%",
        "True_Rushing_Yards",
        "Yards_Lost_From_Sacks",
    ])?;
    
    // Write data
    for qb in qb_stats {
        writer.write_record(&[
            &qb.player,
            &qb.team,
            &qb.opponent,
            &qb.week.to_string(),
            &qb.passing_attempts.to_string(),
            &qb.completions.to_string(),
            &qb.passing_yards.to_string(),
            &format!("{:.2}", qb.yards_per_attempt),
            &qb.passing_tds.to_string(),
            &qb.interceptions.to_string(),
            &format!("{:.2}", qb.int_rate),
            &qb.sacks.to_string(),
            &format!("{:.2}", qb.sack_rate),
            &qb.true_rushing_yards.to_string(),
            &qb.yards_lost_from_sacks.to_string(),
        ])?;
    }
    
    writer.flush()?;
    println!("QB stats written to: {}", filename);
    Ok(())
}

// Get exact sack data for QBs from plays API
async fn get_qb_sacks_from_plays_api(
    team: &str,
    year: u32, 
    week: u32
) -> Result<HashMap<String, (u32, i32)>, Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use serde_json::Value;
    
    let client = reqwest::Client::new();
    let api_key = std::env::var("API_KEY")?;
    
    let url = format!(
        "https://api.collegefootballdata.com/plays?year={}&week={}&team={}", 
        year, week, team
    );
    
    println!("Making plays API request to: {}", url);
    
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("API request failed with status: {}", response.status()).into());
    }
    
    let plays: Vec<Value> = response.json().await?;
    let mut qb_sacks: HashMap<String, (u32, i32)> = HashMap::new();
    
    for play in plays {
        // Check if this is a sack play where our team was on offense
        if let (Some(play_type), Some(play_text), Some(yards_gained), Some(offense)) = (
            play["playType"].as_str(),
            play["playText"].as_str(), 
            play["yardsGained"].as_i64(),
            play["offense"].as_str()
        ) {
            if play_type == "Sack" && offense.to_lowercase().contains(&team.to_lowercase()) {
                // Extract QB name from play text (e.g., "Davis Warren sacked by...")
                if let Some(qb_name) = extract_qb_name_from_sack_text(play_text) {
                    let entry = qb_sacks.entry(qb_name).or_insert((0, 0));
                    entry.0 += 1; // sack count
                    entry.1 += yards_gained as i32; // yards lost (negative)
                }
            }
        }
    }
    
    println!("Found sack data for {} QBs from plays API", qb_sacks.len());
    for (qb, (sacks, yards)) in &qb_sacks {
        println!("  {}: {} sacks for {} yards", qb, sacks, yards);
    }
    
    Ok(qb_sacks)
}

// Extract QB name from sack play text
fn extract_qb_name_from_sack_text(play_text: &str) -> Option<String> {
    // Play text format: "Davis Warren sacked by..."
    if let Some(sacked_pos) = play_text.find(" sacked") {
        let qb_name = play_text[..sacked_pos].trim();
        // Handle cases like "QB Name sacked" vs other formats
        if !qb_name.is_empty() && !qb_name.chars().any(|c| c.is_numeric()) {
            return Some(qb_name.to_string());
        }
    }
    None
}

pub async fn analyze_team_qb_stats(
    team: &str,
    year: u32,
    week: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = ApiClient::new(API_BASE_URL.to_string())?;
    fetch_and_analyze_qb_stats(&client, team, year, week).await?;
    Ok(())
}
