// API endpoints
pub const TEAMS_ENDPOINT: &str = "/teams";
pub const GAME_STATS_ENDPOINT: &str = "/games/players";
pub const PLAYERS_ENDPOINT: &str = "/roster";

// API configuration
pub const API_BASE_URL: &str = "https://api.collegefootballdata.com";
pub const REQUEST_TIMEOUT_SECONDS: u64 = 30;

// File paths
pub const OUTPUT_DIR: &str = "./output";
pub const TEAMS_CSV: &str = "teams.csv";
pub const PLAYERS_CSV: &str = "players.csv";

// Other constants
pub const MAX_RETRIES: u32 = 3;
pub const RATE_LIMIT_DELAY_MS: u64 = 1000;