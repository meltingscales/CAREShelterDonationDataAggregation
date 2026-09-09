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

/// Detection helpers for the "Combine Foster/Adopter" feature.
/// Lets uploads of arbitrarily-shaped adopter/foster export sheets be
/// auto-classified and have their email/name identity columns located,
/// without hardcoding a single expected header name.
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListType {
    Adopter,
    Foster,
    Unknown,
}

impl ListType {
    pub fn label(&self) -> &'static str {
        match self {
            ListType::Adopter => "Adopter",
            ListType::Foster => "Foster",
            ListType::Unknown => "Unknown",
        }
    }
}

/// A single parsed worksheet from an uploaded file, before column selection.
#[derive(Debug, Clone)]
pub struct ParsedSheet {
    pub file_name: String,
    pub sheet_name: String,
    pub list_type: ListType,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

// Marker headers (lowercase) distinctive of each list type, taken from real
// CARE exports. Adopter markers are all "outcome to ..."-prefixed and don't
// collide with foster's bare "city"/"state"/"zip"/"street address 1".
const ADOPTER_MARKERS: &[&str] = &[
    "species",
    "current location",
    "current status",
    "attributes",
    "outcome date",
    "outcome type",
    "outcome to person name",
    "outcome to street address 1",
    "outcome to city",
    "outcome to state",
    "outcome to zip",
    "outcome to phone",
    "outcome to email",
];

const FOSTER_MARKERS: &[&str] = &[
    "date created",
    "person id",
    "street address 1",
    "city",
    "state",
    "zip",
    "primary phone",
    "primary email",
];

fn normalized_header_set(headers: &[String]) -> HashSet<String> {
    headers.iter().map(|h| h.trim().to_lowercase()).collect()
}

/// Detect whether a sheet's headers look like an Adopter list or a Foster
/// list, by counting overlap against known marker headers for each type.
pub fn detect_list_type(headers: &[String]) -> ListType {
    let header_set = normalized_header_set(headers);

    let adopter_score = ADOPTER_MARKERS
        .iter()
        .filter(|m| header_set.contains(**m))
        .count();
    let foster_score = FOSTER_MARKERS
        .iter()
        .filter(|m| header_set.contains(**m))
        .count();

    if adopter_score == 0 && foster_score == 0 {
        ListType::Unknown
    } else if adopter_score >= foster_score {
        ListType::Adopter
    } else {
        ListType::Foster
    }
}

/// Find the index of a header that looks like an email column, e.g.
/// "Outcome To Email" or "Primary Email" both match via "contains email".
pub fn find_email_column_index(headers: &[String]) -> Option<usize> {
    headers
        .iter()
        .position(|h| h.to_lowercase().contains("email"))
}

/// Find the index of a header that looks like a person's full name column.
/// Priority: header containing both "person" and "name" (e.g. "Outcome To
/// Person Name") beats a bare "name" column (which may be an animal's name
/// on an adopter sheet), which beats any other header merely containing
/// "name".
pub fn find_full_name_column_index(headers: &[String]) -> Option<usize> {
    let lower: Vec<String> = headers.iter().map(|h| h.trim().to_lowercase()).collect();

    lower
        .iter()
        .position(|h| h.contains("person") && h.contains("name"))
        .or_else(|| lower.iter().position(|h| h == "name"))
        .or_else(|| lower.iter().position(|h| h.contains("name")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adopter_headers() -> Vec<String> {
        vec![
            "Name", "Species", "Current Location", "Current Status", "Attributes",
            "Outcome Date", "Outcome Type", "Outcome To Person Name",
            "Outcome To Street Address 1", "Outcome To Street Address 2",
            "Outcome To City", "Outcome To State", "Outcome To Zip",
            "Outcome To Phone", "Outcome To Email",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    fn foster_headers() -> Vec<String> {
        vec![
            "Date Created", "Person ID", "Name", "Street Address 1", "Street Address 2",
            "City", "State", "Zip", "Primary Phone", "Primary Email",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    #[test]
    fn test_detect_adopter() {
        assert_eq!(detect_list_type(&adopter_headers()), ListType::Adopter);
    }

    #[test]
    fn test_detect_foster() {
        assert_eq!(detect_list_type(&foster_headers()), ListType::Foster);
    }

    #[test]
    fn test_detect_unknown() {
        let headers = vec!["Amount".to_string(), "Date".to_string(), "Notes".to_string()];
        assert_eq!(detect_list_type(&headers), ListType::Unknown);
    }

    #[test]
    fn test_find_email_adopter() {
        let headers = adopter_headers();
        let idx = find_email_column_index(&headers).unwrap();
        assert_eq!(headers[idx], "Outcome To Email");
    }

    #[test]
    fn test_find_email_foster() {
        let headers = foster_headers();
        let idx = find_email_column_index(&headers).unwrap();
        assert_eq!(headers[idx], "Primary Email");
    }

    #[test]
    fn test_find_name_prefers_person_name_over_pet_name() {
        let headers = adopter_headers();
        let idx = find_full_name_column_index(&headers).unwrap();
        assert_eq!(headers[idx], "Outcome To Person Name");
    }

    #[test]
    fn test_find_name_falls_back_to_bare_name() {
        let headers = foster_headers();
        let idx = find_full_name_column_index(&headers).unwrap();
        assert_eq!(headers[idx], "Name");
    }
}
