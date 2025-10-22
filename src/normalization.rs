// C.A.R.E. Shelter Donation Data Aggregation
// Copyright (C) 2025 Henry Post
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

/// Normalization functions for cleaning and standardizing data

use std::collections::HashMap;

/// Normalizes phone numbers to digits only, then adds two hyphens
/// Example: "(123) 456-7890" -> "123-456-7890"
pub fn normalize_phone(phone: &str) -> String {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits.is_empty() {
        return String::new();
    }

    // Format as XXX-XXX-XXXX if we have 10 digits
    if digits.len() == 10 {
        format!("{}-{}-{}", &digits[0..3], &digits[3..6], &digits[6..10])
    } else if digits.len() == 11 && digits.starts_with('1') {
        // Handle numbers with country code (1-XXX-XXX-XXXX)
        format!("{}-{}-{}", &digits[1..4], &digits[4..7], &digits[7..11])
    } else {
        // For other lengths, just return the digits
        digits
    }
}

/// Normalizes state names to 2-character uppercase abbreviations
/// Handles full state names, abbreviations, and common variations
pub fn normalize_state(state: &str) -> String {
    let state_lower = state.trim().to_lowercase();

    // Create a mapping of state names and variations to abbreviations
    let state_map = get_state_map();

    // If already a 2-char abbreviation, just uppercase it
    if state.trim().len() == 2 {
        return state.trim().to_uppercase();
    }

    // Look up the state name
    state_map
        .get(state_lower.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| state.trim().to_uppercase())
}

/// Returns a HashMap mapping state names and common variations to their abbreviations
fn get_state_map() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();

    // States
    map.insert("alabama", "AL");
    map.insert("alaska", "AK");
    map.insert("arizona", "AZ");
    map.insert("arkansas", "AR");
    map.insert("california", "CA");
    map.insert("colorado", "CO");
    map.insert("connecticut", "CT");
    map.insert("delaware", "DE");
    map.insert("florida", "FL");
    map.insert("georgia", "GA");
    map.insert("hawaii", "HI");
    map.insert("idaho", "ID");
    map.insert("illinois", "IL");
    map.insert("indiana", "IN");
    map.insert("iowa", "IA");
    map.insert("kansas", "KS");
    map.insert("kentucky", "KY");
    map.insert("louisiana", "LA");
    map.insert("maine", "ME");
    map.insert("maryland", "MD");
    map.insert("massachusetts", "MA");
    map.insert("michigan", "MI");
    map.insert("minnesota", "MN");
    map.insert("mississippi", "MS");
    map.insert("missouri", "MO");
    map.insert("montana", "MT");
    map.insert("nebraska", "NE");
    map.insert("nevada", "NV");
    map.insert("new hampshire", "NH");
    map.insert("new jersey", "NJ");
    map.insert("new mexico", "NM");
    map.insert("new york", "NY");
    map.insert("north carolina", "NC");
    map.insert("north dakota", "ND");
    map.insert("ohio", "OH");
    map.insert("oklahoma", "OK");
    map.insert("oregon", "OR");
    map.insert("pennsylvania", "PA");
    map.insert("rhode island", "RI");
    map.insert("south carolina", "SC");
    map.insert("south dakota", "SD");
    map.insert("tennessee", "TN");
    map.insert("texas", "TX");
    map.insert("utah", "UT");
    map.insert("vermont", "VT");
    map.insert("virginia", "VA");
    map.insert("washington", "WA");
    map.insert("west virginia", "WV");
    map.insert("wisconsin", "WI");
    map.insert("wyoming", "WY");

    // Territories and Districts
    map.insert("district of columbia", "DC");
    map.insert("washington dc", "DC");
    map.insert("washington d.c.", "DC");
    map.insert("puerto rico", "PR");
    map.insert("guam", "GU");
    map.insert("american samoa", "AS");
    map.insert("u.s. virgin islands", "VI");
    map.insert("virgin islands", "VI");
    map.insert("northern mariana islands", "MP");

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_phone() {
        assert_eq!(normalize_phone("(123) 456-7890"), "123-456-7890");
        assert_eq!(normalize_phone("123.456.7890"), "123-456-7890");
        assert_eq!(normalize_phone("1234567890"), "123-456-7890");
        assert_eq!(normalize_phone("11234567890"), "123-456-7890");
        assert_eq!(normalize_phone("+1 (123) 456-7890"), "123-456-7890");
        assert_eq!(normalize_phone(""), "");
        assert_eq!(normalize_phone("123"), "123");
    }

    #[test]
    fn test_normalize_state() {
        assert_eq!(normalize_state("California"), "CA");
        assert_eq!(normalize_state("california"), "CA");
        assert_eq!(normalize_state("CALIFORNIA"), "CA");
        assert_eq!(normalize_state("ca"), "CA");
        assert_eq!(normalize_state("CA"), "CA");
        assert_eq!(normalize_state("New York"), "NY");
        assert_eq!(normalize_state("Illinois"), "IL");
        assert_eq!(normalize_state("District of Columbia"), "DC");
        assert_eq!(normalize_state("Puerto Rico"), "PR");
        assert_eq!(normalize_state("  Texas  "), "TX");
        assert_eq!(normalize_state("Unknown State"), "UNKNOWN STATE");
    }
}
