use crate::api::ApiClient;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;

#[derive(Debug, Deserialize)]
struct ApiTeam {
    id: u32,
    school: String,
    conference: Option<String>,
}

pub async fn fetch_and_generate_teams() -> Result<(), Box<dyn std::error::Error>> {
    let client = ApiClient::new("https://api.collegefootballdata.com".to_string())?;
    
    let teams: Vec<ApiTeam> = client.get("/teams").await?;
    
    // Create output file
    let mut file = File::create("college_teams.rs")?;
    
    // Write Rust code for the const array to file
    writeln!(file, "pub const COLLEGE_TEAMS: &[Team] = &[")?;
    for team in teams {
        let conference = team.conference.unwrap_or_else(|| "Independent".to_string());
        writeln!(file, "    Team {{")?;
        writeln!(file, "        id: {},", team.id)?;
        writeln!(file, "        name: \"{}\",", team.school.replace('"', "\\\""))?;
        writeln!(file, "        conference: \"{}\",", conference.replace('"', "\\\""))?;
        writeln!(file, "    }},")?;
    }
    writeln!(file, "];")?;
    
    println!("Teams written to college_teams.rs");
    Ok(())
}
