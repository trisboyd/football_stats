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
    println!("Found game: {} vs {}", game.home_team, game.away_team);
    
    process_game_response(&game.teams, team, week)?;
    
    Ok(())
}

fn process_game_response(
    teams: &[ApiTeamStats],
    target_team: &str,
    week: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut qb_stats_list = Vec::new();
    
    let mut target_team_stats = None;
    let mut opponent_name = String::new();
    
    for team_stats in teams {
        if team_stats.school.to_lowercase().contains(&target_team.to_lowercase()) {
            target_team_stats = Some(team_stats);
        } else {
            opponent_name = team_stats.school.clone();
        }
    }
    
    let target_stats = match target_team_stats {
        Some(stats) => stats,
        None => {
            println!("Could not find stats for team: {}", target_team);
            return Ok(());
        }
    };
    
    println!("Processing stats for {} vs {}", target_stats.school, opponent_name);
    
    let qb_stats = extract_qb_stats_from_api_data(target_stats, &opponent_name, week);
    qb_stats_list.extend(qb_stats);
    
    if qb_stats_list.is_empty() {
        println!("No QB stats found for {}", target_team);
    } else {
        println!("Found {} QB records", qb_stats_list.len());
    }
    
    write_qb_stats_to_csv(&qb_stats_list, &target_stats.school, week, &opponent_name)?;
    
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

pub async fn analyze_team_qb_stats(
    team: &str,
    year: u32,
    week: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = ApiClient::new(API_BASE_URL.to_string())?;
    fetch_and_analyze_qb_stats(&client, team, year, week).await?;
    Ok(())
}
