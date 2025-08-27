#!/usr/bin/env python3

import re
import sys

# List of requested teams
requested_teams = [
    "Alabama", "Clemson", "Ohio State", "Georgia", "Oklahoma", "LSU", "Michigan", 
    "Notre Dame", "Oregon", "Florida", "Penn State", "Wisconsin", "Washington", 
    "USC", "Texas", "Auburn", "Florida State", "Stanford", "Oklahoma State", "UCF", 
    "TCU", "Baylor", "Miami (FL)", "Iowa", "Utah", "Kansas State", "Texas A&M", 
    "Tennessee", "North Carolina", "West Virginia", "Boise State", "BYU", 
    "Virginia Tech", "Mississippi State", "Louisville", "NC State", "Kentucky", 
    "Michigan State", "Arkansas", "California", "UCLA", "Purdue", "Minnesota", 
    "Arizona", "Arizona State", "Pittsburgh", "Coastal Carolina", "Cincinnati", 
    "Houston", "South Carolina", "Syracuse", "Wake Forest", "Boston College", 
    "Marshall", "San Diego State", "Troy", "Memphis", "App State", "Air Force", 
    "Navy", "Wyoming", "Colorado", "Nebraska", "SMU", "Ole Miss", "Missouri", 
    "Indiana", "South Alabama", "Central Florida"
]

# Read the college_teams.rs file
with open('src/college_teams.rs', 'r') as f:
    content = f.read()

# Pattern to match Team structs
team_pattern = r'Team\s*\{\s*id:\s*(\d+),\s*name:\s*"([^"]+)",\s*conference:\s*"([^"]+)",\s*\}'

matches = re.findall(team_pattern, content)

found_teams = []
requested_set = set(requested_teams)

for match in matches:
    id_val, name, conference = match
    if name in requested_set:
        found_teams.append((int(id_val), name, conference))

# Sort by name for consistency
found_teams.sort(key=lambda x: x[1])

print("use crate::models::Team;")
print("")
print("pub const RELEVANT_COLLEGE_TEAMS: &[Team] = &[")

for id_val, name, conference in found_teams:
    print(f'    Team {{')
    print(f'        id: {id_val},')
    print(f'        name: "{name}",')
    print(f'        conference: "{conference}",')
    print(f'    }},')

print("];")

print(f"\n// Found {len(found_teams)} teams out of {len(requested_teams)} requested")

# Print missing teams
found_names = {team[1] for team in found_teams}
missing = [name for name in requested_teams if name not in found_names]
if missing:
    print("// Missing teams:")
    for name in missing:
        print(f"//   {name}")
