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

use regex::Regex;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize)]
pub struct NameSplitResult {
    pub first_name: String,
    pub last_name: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NameSplitAlgorithm {
    pub name: String,
    pub description: String,
    pub first_name_column: String,
    pub last_name_column: String,
}

/// Get all available name splitting algorithms
pub fn get_algorithms() -> Vec<NameSplitAlgorithm> {
    vec![
        NameSplitAlgorithm {
            name: "SplitBySpace".to_string(),
            description: "Splits by first space: 'Alice Smith' → First='Alice', Last='Smith'".to_string(),
            first_name_column: "SplitBySpace_FirstName".to_string(),
            last_name_column: "SplitBySpace_LastName".to_string(),
        },
        NameSplitAlgorithm {
            name: "SplitByLastSpace".to_string(),
            description: "Splits by last space: 'Alice B. Smith' → First='Alice B.', Last='Smith'".to_string(),
            first_name_column: "SplitByLastSpace_FirstName".to_string(),
            last_name_column: "SplitByLastSpace_LastName".to_string(),
        },
        NameSplitAlgorithm {
            name: "SplitByParentheses".to_string(),
            description: "Extracts from parentheses: 'Alice (Annie) Smith' → First='Alice', Last='Smith'".to_string(),
            first_name_column: "SplitByParentheses_FirstName".to_string(),
            last_name_column: "SplitByParentheses_LastName".to_string(),
        },
        NameSplitAlgorithm {
            name: "SplitByParenthesesNickname".to_string(),
            description: "Uses nickname in parentheses: 'Alice (Annie) Smith' → First='Annie', Last='Smith'".to_string(),
            first_name_column: "SplitByParenthesesNickname_FirstName".to_string(),
            last_name_column: "SplitByParenthesesNickname_LastName".to_string(),
        },
        NameSplitAlgorithm {
            name: "SplitByAnd".to_string(),
            description: "Splits by 'and': 'Alice and Bob Smith' → First='Alice and Bob', Last='Smith'".to_string(),
            first_name_column: "SplitByAnd_FirstName".to_string(),
            last_name_column: "SplitByAnd_LastName".to_string(),
        },
        NameSplitAlgorithm {
            name: "SplitByAndFirstOnly".to_string(),
            description: "Takes first name before 'and': 'Alice and Bob Smith' → First='Alice', Last='Smith'".to_string(),
            first_name_column: "SplitByAndFirstOnly_FirstName".to_string(),
            last_name_column: "SplitByAndFirstOnly_LastName".to_string(),
        },
        NameSplitAlgorithm {
            name: "SplitByHyphen".to_string(),
            description: "Splits by hyphen: 'Alice-Smith' → First='Alice', Last='Smith'".to_string(),
            first_name_column: "SplitByHyphen_FirstName".to_string(),
            last_name_column: "SplitByHyphen_LastName".to_string(),
        },
    ]
}

/// Split by first space
pub fn split_by_space(name: &str) -> NameSplitResult {
    let trimmed = name.trim();
    if let Some(space_pos) = trimmed.find(' ') {
        NameSplitResult {
            first_name: trimmed[..space_pos].trim().to_string(),
            last_name: trimmed[space_pos + 1..].trim().to_string(),
        }
    } else {
        NameSplitResult {
            first_name: trimmed.to_string(),
            last_name: String::new(),
        }
    }
}

/// Split by last space (useful for middle names)
pub fn split_by_last_space(name: &str) -> NameSplitResult {
    let trimmed = name.trim();
    if let Some(space_pos) = trimmed.rfind(' ') {
        NameSplitResult {
            first_name: trimmed[..space_pos].trim().to_string(),
            last_name: trimmed[space_pos + 1..].trim().to_string(),
        }
    } else {
        NameSplitResult {
            first_name: trimmed.to_string(),
            last_name: String::new(),
        }
    }
}

/// Extract name from parentheses - uses first name before parentheses, last name after
pub fn split_by_parentheses(name: &str) -> NameSplitResult {
    let re = Regex::new(r"^([^(]+)\s*\(([^)]+)\)\s*(.*)$").unwrap();

    if let Some(caps) = re.captures(name.trim()) {
        let first_name = caps.get(1).map_or("", |m| m.as_str()).trim();
        let last_name = caps.get(3).map_or("", |m| m.as_str()).trim();
        NameSplitResult {
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
        }
    } else {
        // If no parentheses found, fall back to space split
        split_by_space(name)
    }
}

/// Extract nickname from parentheses and use it as first name
pub fn split_by_parentheses_nickname(name: &str) -> NameSplitResult {
    let re = Regex::new(r"^[^(]+\s*\(([^)]+)\)\s*(.*)$").unwrap();

    if let Some(caps) = re.captures(name.trim()) {
        let nickname = caps.get(1).map_or("", |m| m.as_str()).trim();
        let last_name = caps.get(2).map_or("", |m| m.as_str()).trim();
        NameSplitResult {
            first_name: nickname.to_string(),
            last_name: last_name.to_string(),
        }
    } else {
        // If no parentheses found, fall back to space split
        split_by_space(name)
    }
}

/// Split by "and" - keeps both names together
pub fn split_by_and(name: &str) -> NameSplitResult {
    let re = Regex::new(r"(?i)\s+and\s+").unwrap();

    if re.is_match(name) {
        let parts: Vec<&str> = name.split_whitespace().collect();
        if parts.len() >= 3 {
            // Last word is typically the last name
            let last_name = parts.last().unwrap_or(&"");
            // Everything before the last word is the first name(s)
            let first_name = parts[..parts.len()-1].join(" ");
            return NameSplitResult {
                first_name,
                last_name: last_name.to_string(),
            };
        }
    }

    // If no "and" found, fall back to space split
    split_by_space(name)
}

/// Split by "and" - takes only the first name before "and"
pub fn split_by_and_first_only(name: &str) -> NameSplitResult {
    let re = Regex::new(r"(?i)\s+and\s+").unwrap();

    if re.is_match(name) {
        let parts: Vec<&str> = name.split_whitespace().collect();
        if parts.len() >= 3 {
            // First word is the first person's first name
            let first_name = parts.first().unwrap_or(&"");
            // Last word is typically the last name
            let last_name = parts.last().unwrap_or(&"");
            return NameSplitResult {
                first_name: first_name.to_string(),
                last_name: last_name.to_string(),
            };
        }
    }

    // If no "and" found, fall back to space split
    split_by_space(name)
}

/// Split by hyphen
pub fn split_by_hyphen(name: &str) -> NameSplitResult {
    let trimmed = name.trim();
    if let Some(hyphen_pos) = trimmed.find('-') {
        NameSplitResult {
            first_name: trimmed[..hyphen_pos].trim().to_string(),
            last_name: trimmed[hyphen_pos + 1..].trim().to_string(),
        }
    } else {
        NameSplitResult {
            first_name: trimmed.to_string(),
            last_name: String::new(),
        }
    }
}

/// Apply all algorithms to a name
pub fn apply_all_algorithms(name: &str) -> Vec<(String, NameSplitResult)> {
    vec![
        ("SplitBySpace".to_string(), split_by_space(name)),
        ("SplitByLastSpace".to_string(), split_by_last_space(name)),
        ("SplitByParentheses".to_string(), split_by_parentheses(name)),
        ("SplitByParenthesesNickname".to_string(), split_by_parentheses_nickname(name)),
        ("SplitByAnd".to_string(), split_by_and(name)),
        ("SplitByAndFirstOnly".to_string(), split_by_and_first_only(name)),
        ("SplitByHyphen".to_string(), split_by_hyphen(name)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_by_space() {
        let result = split_by_space("Alice Smith");
        assert_eq!(result.first_name, "Alice");
        assert_eq!(result.last_name, "Smith");
    }

    #[test]
    fn test_split_by_last_space() {
        let result = split_by_last_space("Alice B. Smith");
        assert_eq!(result.first_name, "Alice B.");
        assert_eq!(result.last_name, "Smith");
    }

    #[test]
    fn test_split_by_parentheses() {
        let result = split_by_parentheses("Alice (Annie) Smith");
        assert_eq!(result.first_name, "Alice");
        assert_eq!(result.last_name, "Smith");
    }

    #[test]
    fn test_split_by_hyphen() {
        let result = split_by_hyphen("Alice-Smith");
        assert_eq!(result.first_name, "Alice");
        assert_eq!(result.last_name, "Smith");
    }
}
