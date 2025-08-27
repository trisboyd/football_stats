use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Team {
    pub id: u32,
    pub name: &'static str,
    pub conference: &'static str,
}

// Static team data - these won't change
pub const COLLEGE_TEAMS: &[Team] = &[
];

#[derive(Debug, Deserialize, Serialize)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub position: String,
    pub team_id: u32,
    // Add other fields as needed
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Game {
    pub id: u32,
    #[serde(rename = "season")]
    pub year: u32,
    pub week: u32,
    #[serde(rename = "season_type")]
    pub season_type: String,
    #[serde(rename = "home_team")]
    pub home_team: String,
    #[serde(rename = "away_team")]
    pub away_team: String,
    #[serde(rename = "home_points")]
    pub home_points: Option<u32>,
    #[serde(rename = "away_points")]
    pub away_points: Option<u32>,
    pub teams: Vec<ApiTeamStats>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTeamStats {
    pub school: String,
    pub conference: Option<String>,
    pub home_away: Option<String>,
    pub points: Option<u32>,
    pub stats: Vec<ApiStatCategory>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiStatCategory {
    pub name: String,
    pub types: Vec<ApiStatType>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiStatType {
    pub name: String,
    pub athletes: Vec<ApiAthleteStat>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiAthleteStat {
    pub id: String,
    pub name: String,
    pub stat: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct QBStats {
    pub player: String,
    pub team: String,
    pub opponent: String,
    pub week: u32,
    pub passing_attempts: i32,
    pub completions: i32,
    pub passing_yards: i32,
    pub yards_per_attempt: f64,
    pub passing_tds: i32,
    pub interceptions: i32,
    pub int_rate: f64,
    pub sacks: i32,
    pub sack_rate: f64,
    pub true_rushing_yards: i32,
    pub yards_lost_from_sacks: i32,
}
