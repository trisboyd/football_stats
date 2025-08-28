use crate::models::{ApiTeamStats, RunningBackStats, ReceiverStats, DefensiveStats};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use csv::Writer;

// Helper function to create directory structure and return the path
fn ensure_output_directory(team: &str, week: u32) -> Result<String, Box<dyn Error>> {
    let base_dir = format!("output/{}/week_{}", team, week);
    fs::create_dir_all(&base_dir)?;
    Ok(base_dir)
}

pub async fn analyze_all_position_stats(
    teams: &[ApiTeamStats],
    target_team: &str,
    year: u32,
    week: u32,
) -> Result<(), Box<dyn Error>> {
    
    // Find opponent name for display purposes
    let opponent_name = teams.iter()
        .find(|team_stats| !team_stats.school.to_lowercase().contains(&target_team.to_lowercase()))
        .map(|stats| stats.school.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    println!("\n🏈 Analyzing ALL POSITION STATS for {} vs {} Week {}", target_team, opponent_name, week);
    
    // Ensure output directory exists
    let output_dir = ensure_output_directory(target_team, week)?;

    // Process Running Backs
    let rb_stats = extract_running_back_stats(teams, target_team, &opponent_name, week);
    if !rb_stats.is_empty() {
        write_running_back_stats_to_csv(&rb_stats, &opponent_name, week, &output_dir)?;
        println!("✅ Generated {} running back records", rb_stats.len());
    }

    // Process Wide Receivers and Tight Ends  
    let wr_te_stats = extract_receiver_stats(teams, target_team, &opponent_name, week);
    if !wr_te_stats.is_empty() {
        write_receiver_stats_to_csv(&wr_te_stats, &opponent_name, week, &output_dir)?;
        println!("✅ Generated {} receiver records", wr_te_stats.len());
    }

    // Process Defensive Players
    let def_stats = extract_defensive_stats(teams, target_team, &opponent_name, week);
    if !def_stats.is_empty() {
        write_defensive_stats_to_csv(&def_stats, &opponent_name, week, &output_dir)?;
        println!("✅ Generated {} defensive player records", def_stats.len());
    }

    Ok(())
}

// Extract Running Back Stats (rushing + receiving)
fn extract_running_back_stats(
    teams: &[ApiTeamStats], 
    target_team: &str, 
    opponent: &str, 
    week: u32
) -> Vec<RunningBackStats> {
    let mut rb_data: HashMap<String, RBStatsBuilder> = HashMap::new();
    
    for team_stats in teams {
        if !team_stats.school.to_lowercase().contains(&target_team.to_lowercase()) {
            continue;
        }
        
        // Process rushing, receiving, and fumbles stats
        for stat_category in &team_stats.stats {
            match stat_category.name.as_str() {
                "rushing" => process_rushing_for_rbs(&stat_category.types, &mut rb_data),
                "receiving" => process_receiving_for_rbs(&stat_category.types, &mut rb_data),
                "fumbles" => process_fumbles_for_rbs(&stat_category.types, &mut rb_data),
                _ => {}
            }
        }
        
        // Convert builders to final RB stats
        return rb_data.into_iter()
            .filter_map(|(player_name, builder)| {
                builder.build(&player_name, &team_stats.school, opponent, week)
            })
            .collect();
    }
    
    Vec::new()
}

// Extract Receiver Stats (WR/TE)
fn extract_receiver_stats(
    teams: &[ApiTeamStats], 
    target_team: &str, 
    opponent: &str, 
    week: u32
) -> Vec<ReceiverStats> {
    let mut wr_data: HashMap<String, WRStatsBuilder> = HashMap::new();
    
    for team_stats in teams {
        if !team_stats.school.to_lowercase().contains(&target_team.to_lowercase()) {
            continue;
        }
        
        // Process receiving and fumbles stats
        for stat_category in &team_stats.stats {
            match stat_category.name.as_str() {
                "receiving" => process_receiving_for_wrs(&stat_category.types, &mut wr_data),
                "fumbles" => process_fumbles_for_wrs(&stat_category.types, &mut wr_data),
                _ => {}
            }
        }
        
        // Convert builders to final WR stats - only include players with receptions
        return wr_data.into_iter()
            .filter_map(|(player_name, builder)| {
                if builder.receptions > 0 {
                    builder.build(&player_name, &team_stats.school, opponent, week)
                } else {
                    None
                }
            })
            .collect();
    }
    
    Vec::new()
}

// Extract Defensive Stats
fn extract_defensive_stats(
    teams: &[ApiTeamStats], 
    target_team: &str, 
    opponent: &str, 
    week: u32
) -> Vec<DefensiveStats> {
    let mut def_data: HashMap<String, DefStatsBuilder> = HashMap::new();
    
    for team_stats in teams {
        if !team_stats.school.to_lowercase().contains(&target_team.to_lowercase()) {
            continue;
        }
        
        // Process defensive, interceptions, and fumbles stats
        for stat_category in &team_stats.stats {
            match stat_category.name.as_str() {
                "defensive" => process_defensive_for_def(&stat_category.types, &mut def_data),
                "interceptions" => process_interceptions_for_def(&stat_category.types, &mut def_data),
                "fumbles" => process_fumbles_for_def(&stat_category.types, &mut def_data),
                _ => {}
            }
        }
        
        // Convert builders to final defensive stats - only include players with defensive activity
        return def_data.into_iter()
            .filter_map(|(player_name, builder)| {
                if builder.has_defensive_activity() {
                    builder.build(&player_name, &team_stats.school, opponent, week)
                } else {
                    None
                }
            })
            .collect();
    }
    
    Vec::new()
}

// Builder structures for accumulating stats
#[derive(Default)]
struct RBStatsBuilder {
    rushing_attempts: u32,
    rushing_yards: i32,
    rushing_tds: u32,
    longest_rush: u32,
    receptions: u32,
    receiving_yards: i32,
    receiving_tds: u32,
    longest_reception: u32,
    fumbles: u32,
    fumbles_lost: u32,
}

impl RBStatsBuilder {
    fn build(self, player: &str, team: &str, opponent: &str, week: u32) -> Option<RunningBackStats> {
        // Only include players with rushing attempts or receptions 
        if self.rushing_attempts == 0 && self.receptions == 0 {
            return None;
        }
        
        let yards_per_carry = if self.rushing_attempts > 0 {
            self.rushing_yards as f64 / self.rushing_attempts as f64
        } else {
            0.0
        };
        
        let yards_per_reception = if self.receptions > 0 {
            self.receiving_yards as f64 / self.receptions as f64
        } else {
            0.0
        };
        
        let all_purpose_yards = self.rushing_yards + self.receiving_yards;
        
        Some(RunningBackStats {
            player: player.to_string(),
            team: team.to_string(),
            opponent: opponent.to_string(),
            week,
            rushing_attempts: self.rushing_attempts,
            rushing_yards: self.rushing_yards,
            yards_per_carry,
            rushing_touchdowns: self.rushing_tds,
            longest_rush: self.longest_rush,
            receptions: self.receptions,
            receiving_yards: self.receiving_yards,
            yards_per_reception,
            receiving_touchdowns: self.receiving_tds,
            longest_reception: self.longest_reception,
            all_purpose_yards,
            fumbles: self.fumbles,
            fumbles_lost: self.fumbles_lost,
        })
    }
}

#[derive(Default)]
struct WRStatsBuilder {
    receptions: u32,
    receiving_yards: i32,
    receiving_tds: u32,
    longest_reception: u32,
    fumbles: u32,
    fumbles_lost: u32,
}

impl WRStatsBuilder {
    fn build(self, player: &str, team: &str, opponent: &str, week: u32) -> Option<ReceiverStats> {
        let yards_per_reception = if self.receptions > 0 {
            self.receiving_yards as f64 / self.receptions as f64
        } else {
            0.0
        };
        
        Some(ReceiverStats {
            player: player.to_string(),
            team: team.to_string(),
            opponent: opponent.to_string(),
            week,
            receptions: self.receptions,
            receiving_yards: self.receiving_yards,
            yards_per_reception,
            receiving_touchdowns: self.receiving_tds,
            longest_reception: self.longest_reception,
            fumbles: self.fumbles,
            fumbles_lost: self.fumbles_lost,
        })
    }
}

#[derive(Default)]
struct DefStatsBuilder {
    total_tackles: f64,
    solo_tackles: f64,
    tackles_for_loss: f64,
    sacks: f64,
    quarterback_hurries: f64,
    pass_breakups: f64,
    interceptions: u32,
    interception_return_yards: i32,
    interception_return_tds: u32,
    fumbles_recovered: f64,
    defensive_touchdowns: u32,
}

impl DefStatsBuilder {
    fn has_defensive_activity(&self) -> bool {
        self.total_tackles > 0.0 || 
        self.sacks > 0.0 || 
        self.pass_breakups > 0.0 || 
        self.interceptions > 0 ||
        self.fumbles_recovered > 0.0
    }
    
    fn build(self, player: &str, team: &str, opponent: &str, week: u32) -> Option<DefensiveStats> {
        Some(DefensiveStats {
            player: player.to_string(),
            team: team.to_string(),
            opponent: opponent.to_string(),
            week,
            total_tackles: self.total_tackles,
            solo_tackles: self.solo_tackles,
            tackles_for_loss: self.tackles_for_loss,
            sacks: self.sacks,
            quarterback_hurries: self.quarterback_hurries,
            pass_breakups: self.pass_breakups,
            interceptions: self.interceptions,
            interception_return_yards: self.interception_return_yards,
            interception_return_tds: self.interception_return_tds,
            fumbles_recovered: self.fumbles_recovered,
            defensive_touchdowns: self.defensive_touchdowns,
        })
    }
}

// Processing functions for stats
fn process_rushing_for_rbs(stat_types: &[crate::models::ApiStatType], rb_data: &mut HashMap<String, RBStatsBuilder>) {
    for stat_type in stat_types {
        for athlete_stat in &stat_type.athletes {
            let builder = rb_data.entry(athlete_stat.name.clone()).or_default();
            let value: f64 = athlete_stat.stat.parse().unwrap_or(0.0);
            
            match stat_type.name.as_str() {
                "CAR" => builder.rushing_attempts = value as u32,
                "YDS" => builder.rushing_yards = value as i32,
                "TD" => builder.rushing_tds = value as u32,
                "LONG" => builder.longest_rush = value as u32,
                _ => {}
            }
        }
    }
}

fn process_receiving_for_rbs(stat_types: &[crate::models::ApiStatType], rb_data: &mut HashMap<String, RBStatsBuilder>) {
    for stat_type in stat_types {
        for athlete_stat in &stat_type.athletes {
            let builder = rb_data.entry(athlete_stat.name.clone()).or_default();
            let value: f64 = athlete_stat.stat.parse().unwrap_or(0.0);
            
            match stat_type.name.as_str() {
                "REC" => builder.receptions = value as u32,
                "YDS" => builder.receiving_yards = value as i32,
                "TD" => builder.receiving_tds = value as u32,
                "LONG" => builder.longest_reception = value as u32,
                _ => {}
            }
        }
    }
}

fn process_fumbles_for_rbs(stat_types: &[crate::models::ApiStatType], rb_data: &mut HashMap<String, RBStatsBuilder>) {
    for stat_type in stat_types {
        for athlete_stat in &stat_type.athletes {
            let builder = rb_data.entry(athlete_stat.name.clone()).or_default();
            let value: f64 = athlete_stat.stat.parse().unwrap_or(0.0);
            
            match stat_type.name.as_str() {
                "FUM" => builder.fumbles = value as u32,
                "LOST" => builder.fumbles_lost = value as u32,
                _ => {}
            }
        }
    }
}

fn process_receiving_for_wrs(stat_types: &[crate::models::ApiStatType], wr_data: &mut HashMap<String, WRStatsBuilder>) {
    for stat_type in stat_types {
        for athlete_stat in &stat_type.athletes {
            let builder = wr_data.entry(athlete_stat.name.clone()).or_default();
            let value: f64 = athlete_stat.stat.parse().unwrap_or(0.0);
            
            match stat_type.name.as_str() {
                "REC" => builder.receptions = value as u32,
                "YDS" => builder.receiving_yards = value as i32,
                "TD" => builder.receiving_tds = value as u32,
                "LONG" => builder.longest_reception = value as u32,
                _ => {}
            }
        }
    }
}

fn process_fumbles_for_wrs(stat_types: &[crate::models::ApiStatType], wr_data: &mut HashMap<String, WRStatsBuilder>) {
    for stat_type in stat_types {
        for athlete_stat in &stat_type.athletes {
            let builder = wr_data.entry(athlete_stat.name.clone()).or_default();
            let value: f64 = athlete_stat.stat.parse().unwrap_or(0.0);
            
            match stat_type.name.as_str() {
                "FUM" => builder.fumbles = value as u32,
                "LOST" => builder.fumbles_lost = value as u32,
                _ => {}
            }
        }
    }
}

fn process_defensive_for_def(stat_types: &[crate::models::ApiStatType], def_data: &mut HashMap<String, DefStatsBuilder>) {
    for stat_type in stat_types {
        for athlete_stat in &stat_type.athletes {
            let builder = def_data.entry(athlete_stat.name.clone()).or_default();
            let value: f64 = athlete_stat.stat.parse().unwrap_or(0.0);
            
            match stat_type.name.as_str() {
                "TOT" => builder.total_tackles = value,
                "SOLO" => builder.solo_tackles = value,
                "TFL" => builder.tackles_for_loss = value,
                "SACKS" => builder.sacks = value,
                "QB HUR" => builder.quarterback_hurries = value as f64,
                "PD" => builder.pass_breakups = value,
                "TD" => builder.defensive_touchdowns = value as u32,
                _ => {}
            }
        }
    }
}

fn process_interceptions_for_def(stat_types: &[crate::models::ApiStatType], def_data: &mut HashMap<String, DefStatsBuilder>) {
    for stat_type in stat_types {
        for athlete_stat in &stat_type.athletes {
            let builder = def_data.entry(athlete_stat.name.clone()).or_default();
            let value: f64 = athlete_stat.stat.parse().unwrap_or(0.0);
            
            match stat_type.name.as_str() {
                "INT" => builder.interceptions = value as u32,
                "YDS" => builder.interception_return_yards = value as i32,
                "TD" => builder.interception_return_tds = value as u32,
                _ => {}
            }
        }
    }
}

fn process_fumbles_for_def(stat_types: &[crate::models::ApiStatType], def_data: &mut HashMap<String, DefStatsBuilder>) {
    for stat_type in stat_types {
        for athlete_stat in &stat_type.athletes {
            let builder = def_data.entry(athlete_stat.name.clone()).or_default();
            let value: f64 = athlete_stat.stat.parse().unwrap_or(0.0);
            
            match stat_type.name.as_str() {
                "REC" => builder.fumbles_recovered = value,
                _ => {}
            }
        }
    }
}

// CSV Writers
fn write_running_back_stats_to_csv(
    stats: &[RunningBackStats],
    opponent: &str,
    week: u32,
    output_dir: &str,
) -> Result<(), Box<dyn Error>> {
    let filename = if let Some(first_stat) = stats.first() {
        format!("{}/RB_Stats_{}_vs_{}_Week_{}.csv", output_dir, first_stat.team, opponent, week)
    } else {
        format!("{}/RB_Stats_Week_{}.csv", output_dir, week)
    };

    let mut wtr = Writer::from_path(&filename)?;
    
    // Write headers
    wtr.write_record(&[
        "Player", "Team", "Opponent", "Week", 
        "Rushing_Attempts", "Rushing_Yards", "Yards_Per_Carry", "Rushing_TDs", "Longest_Rush",
        "Receptions", "Receiving_Yards", "Yards_Per_Reception", "Receiving_TDs", "Longest_Reception",
        "All_Purpose_Yards", "Fumbles", "Fumbles_Lost"
    ])?;

    // Write data rows
    for stat in stats {
        wtr.write_record(&[
            &stat.player,
            &stat.team,
            &stat.opponent,
            &stat.week.to_string(),
            &stat.rushing_attempts.to_string(),
            &stat.rushing_yards.to_string(),
            &format!("{:.1}", stat.yards_per_carry),
            &stat.rushing_touchdowns.to_string(),
            &stat.longest_rush.to_string(),
            &stat.receptions.to_string(),
            &stat.receiving_yards.to_string(),
            &format!("{:.1}", stat.yards_per_reception),
            &stat.receiving_touchdowns.to_string(),
            &stat.longest_reception.to_string(),
            &stat.all_purpose_yards.to_string(),
            &stat.fumbles.to_string(),
            &stat.fumbles_lost.to_string(),
        ])?;
    }

    wtr.flush()?;
    println!("📄 Running back stats written to: {}", filename);
    Ok(())
}

fn write_receiver_stats_to_csv(
    stats: &[ReceiverStats],
    opponent: &str,
    week: u32,
    output_dir: &str,
) -> Result<(), Box<dyn Error>> {
    let filename = if let Some(first_stat) = stats.first() {
        format!("{}/WR_TE_Stats_{}_vs_{}_Week_{}.csv", output_dir, first_stat.team, opponent, week)
    } else {
        format!("{}/WR_TE_Stats_Week_{}.csv", output_dir, week)
    };

    let mut wtr = Writer::from_path(&filename)?;
    
    // Write headers
    wtr.write_record(&[
        "Player", "Team", "Opponent", "Week",
        "Receptions", "Receiving_Yards", "Yards_Per_Reception", "Receiving_TDs", "Longest_Reception",
        "Fumbles", "Fumbles_Lost"
    ])?;

    // Write data rows
    for stat in stats {
        wtr.write_record(&[
            &stat.player,
            &stat.team,
            &stat.opponent,
            &stat.week.to_string(),
            &stat.receptions.to_string(),
            &stat.receiving_yards.to_string(),
            &format!("{:.1}", stat.yards_per_reception),
            &stat.receiving_touchdowns.to_string(),
            &stat.longest_reception.to_string(),
            &stat.fumbles.to_string(),
            &stat.fumbles_lost.to_string(),
        ])?;
    }

    wtr.flush()?;
    println!("📄 Receiver stats written to: {}", filename);
    Ok(())
}

fn write_defensive_stats_to_csv(
    stats: &[DefensiveStats],
    opponent: &str,
    week: u32,
    output_dir: &str,
) -> Result<(), Box<dyn Error>> {
    let filename = if let Some(first_stat) = stats.first() {
        format!("{}/Defensive_Stats_{}_vs_{}_Week_{}.csv", output_dir, first_stat.team, opponent, week)
    } else {
        format!("{}/Defensive_Stats_Week_{}.csv", output_dir, week)
    };

    let mut wtr = Writer::from_path(&filename)?;
    
    // Write headers
    wtr.write_record(&[
        "Player", "Team", "Opponent", "Week",
        "Total_Tackles", "Solo_Tackles", "Tackles_For_Loss", "Sacks", "QB_Hurries", "Pass_Breakups",
        "Interceptions", "Interception_Return_Yards", "Interception_Return_TDs", 
        "Fumbles_Recovered", "Defensive_TDs"
    ])?;

    // Write data rows
    for stat in stats {
        wtr.write_record(&[
            &stat.player,
            &stat.team,
            &stat.opponent,
            &stat.week.to_string(),
            &format!("{:.1}", stat.total_tackles),
            &format!("{:.1}", stat.solo_tackles),
            &format!("{:.1}", stat.tackles_for_loss),
            &format!("{:.1}", stat.sacks),
            &format!("{:.1}", stat.quarterback_hurries),
            &format!("{:.1}", stat.pass_breakups),
            &stat.interceptions.to_string(),
            &stat.interception_return_yards.to_string(),
            &stat.interception_return_tds.to_string(),
            &format!("{:.1}", stat.fumbles_recovered),
            &stat.defensive_touchdowns.to_string(),
        ])?;
    }

    wtr.flush()?;
    println!("📄 Defensive stats written to: {}", filename);
    Ok(())
}
