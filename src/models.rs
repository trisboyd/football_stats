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
    pub teams: Vec<ApiTeamStats>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTeamStats {
    #[serde(rename = "team")]
    pub school: String,
    pub conference: Option<String>,
    pub home_away: Option<String>,
    pub points: Option<u32>,
    #[serde(rename = "categories")]
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

// Running Back Stats
#[derive(Debug, Serialize, Clone)]
pub struct RunningBackStats {
    pub player: String,
    pub team: String,
    pub opponent: String,
    pub week: u32,
    pub rushing_attempts: u32,
    pub rushing_yards: i32,
    pub yards_per_carry: f64,
    pub rushing_touchdowns: u32,
    pub longest_rush: u32,
    pub receptions: u32,
    pub receiving_yards: i32,
    pub yards_per_reception: f64,
    pub receiving_touchdowns: u32,
    pub longest_reception: u32,
    pub all_purpose_yards: i32,
    pub fumbles: u32,
    pub fumbles_lost: u32,
}

// Wide Receiver / Tight End Stats
#[derive(Debug, Serialize, Clone)]
pub struct ReceiverStats {
    pub player: String,
    pub team: String,
    pub opponent: String,
    pub week: u32,
    pub receptions: u32,
    pub receiving_yards: i32,
    pub yards_per_reception: f64,
    pub receiving_touchdowns: u32,
    pub longest_reception: u32,
    pub fumbles: u32,
    pub fumbles_lost: u32,
}

// Defensive Player Stats (DL/LB and DB)
#[derive(Debug, Serialize, Clone)]
pub struct DefensiveStats {
    pub player: String,
    pub team: String,
    pub opponent: String,
    pub week: u32,
    pub total_tackles: f64,
    pub solo_tackles: f64,
    pub tackles_for_loss: f64,
    pub sacks: f64,
    pub quarterback_hurries: f64,
    pub pass_breakups: f64,
    pub interceptions: u32,
    pub interception_return_yards: i32,
    pub interception_return_tds: u32,
    pub fumbles_recovered: f64,
    pub defensive_touchdowns: u32,
}
