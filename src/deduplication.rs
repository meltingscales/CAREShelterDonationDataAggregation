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

/// Deduplication logic for merging records based on email addresses

use std::collections::HashMap;
use csv::StringRecord;
use crate::data_mappings::has_higher_priority;

/// Represents the result of a deduplication operation
pub struct DeduplicationResult {
    /// The deduplicated records with their source sheet names
    pub records: Vec<(StringRecord, String)>,
    /// Verbose log of all deduplication operations
    pub log: String,
    /// Any errors encountered during deduplication
    pub errors: Vec<String>,
    /// Number of records before deduplication
    pub records_before_dedup: usize,
    /// Number of duplicates removed
    pub duplicates_removed: usize,
    /// The records that were removed as duplicates (includes sheet source)
    pub removed_duplicates: Vec<(StringRecord, String)>,
}

/// Deduplicates records based on email addresses (case-insensitive)
///
/// Priority rules:
/// 1. Non-empty values overwrite empty values
/// 2. If both non-empty, values from higher priority sheets win (see DEDUPLICATION_PRIORITY in data_mappings.rs)
///
/// # Arguments
/// * `headers` - CSV headers
/// * `records` - All records to deduplicate
/// * `source_sheet_name` - Name of the source sheet (for priority rules)
///
/// # Returns
/// A `DeduplicationResult` containing deduplicated records, logs, and errors
pub fn deduplicate_records(
    headers: &StringRecord,
    records: Vec<StringRecord>,
    source_sheet_name: &str,
) -> DeduplicationResult {
    let mut log = String::new();
    let mut errors = Vec::new();
    let mut email_map: HashMap<String, (StringRecord, String)> = HashMap::new();
    let mut removed_duplicates: Vec<(StringRecord, String)> = Vec::new();

    // Find the email column index
    let email_idx = headers.iter().position(|h| h == "EMail");

    if email_idx.is_none() {
        errors.push("No 'EMail' column found in headers".to_string());
        let records_count = records.len();
        // Convert records to include source sheet
        let records_with_source: Vec<(StringRecord, String)> = records
            .into_iter()
            .map(|r| (r, source_sheet_name.to_string()))
            .collect();
        return DeduplicationResult {
            records: records_with_source,
            log,
            errors,
            records_before_dedup: records_count,
            duplicates_removed: 0,
            removed_duplicates: vec![],
        };
    }

    let email_idx = email_idx.unwrap();

    // Find First and Last name indices for fallback key
    let first_idx = headers.iter().position(|h| h == "First");
    let last_idx = headers.iter().position(|h| h == "Last");

    log.push_str("=== DEDUPLICATION LOG ===\n");
    log.push_str("Use CTRL-F to search for specific emails or names\n\n");
    log.push_str(&format!(
        "Processing {} records from sheet: {}\n\n",
        records.len(),
        source_sheet_name
    ));
    log.push_str("Note: Records with email use email as primary key.\n");
    log.push_str("      Records without email use First+Last name as primary key.\n\n");

    // Process each record
    for (row_num, record) in records.iter().enumerate() {
        let email = match record.get(email_idx) {
            Some(e) => e.trim().to_uppercase(),
            None => String::new(),
        };

        // Determine the deduplication key: email if present, otherwise First+Last
        let dedup_key = if !email.is_empty() {
            email.clone()
        } else {
            // Use First + Last as fallback key
            let first = first_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .trim()
                .to_uppercase();
            let last = last_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .trim()
                .to_uppercase();

            if first.is_empty() && last.is_empty() {
                log.push_str(&format!(
                    "Row {}: Skipping record with no email and no name\n",
                    row_num + 1
                ));
                continue;
            }

            format!("NAME:{}|{}", first, last)
        };

        // Check if we've seen this key before
        if let Some((existing_record, existing_sheet)) = email_map.get(&dedup_key) {
            log.push_str(&format!("\n--- DUPLICATE FOUND ---\n"));
            if dedup_key.starts_with("NAME:") {
                log.push_str(&format!("Name-based key: {}\n", dedup_key.replace("NAME:", "")));
            } else {
                log.push_str(&format!("Email: {}\n", dedup_key));
            }
            log.push_str(&format!("Row {}: New record from sheet '{}'\n", row_num + 1, source_sheet_name));
            log.push_str(&format!("Existing record from sheet '{}'\n", existing_sheet));

            // Store the "loser" in removed duplicates before merging
            // The record that gets discarded is the one from the lower-priority sheet
            if has_higher_priority(source_sheet_name, existing_sheet) {
                // New record has higher priority, existing record is removed
                removed_duplicates.push((existing_record.clone(), existing_sheet.clone()));
            } else {
                // Existing record has higher priority, new record is removed
                removed_duplicates.push((record.clone(), source_sheet_name.to_string()));
            }

            // Merge the records according to priority rules
            let merged = merge_records(
                headers,
                existing_record,
                record,
                existing_sheet,
                source_sheet_name,
                &mut log,
            );

            // Determine which sheet name to keep (higher priority sheet)
            let merged_sheet = if has_higher_priority(source_sheet_name, existing_sheet) {
                source_sheet_name.to_string()
            } else {
                existing_sheet.clone()
            };

            email_map.insert(dedup_key.clone(), (merged, merged_sheet));
        } else {
            if dedup_key.starts_with("NAME:") {
                log.push_str(&format!(
                    "Row {}: New name-based record: {}\n",
                    row_num + 1,
                    dedup_key.replace("NAME:", "")
                ));
            } else {
                log.push_str(&format!("Row {}: New email: {}\n", row_num + 1, dedup_key));
            }
            email_map.insert(dedup_key, (record.clone(), source_sheet_name.to_string()));
        }
    }

    log.push_str(&format!(
        "\n=== DEDUPLICATION COMPLETE ===\n"
    ));
    log.push_str(&format!(
        "Original records: {}\n",
        records.len()
    ));
    log.push_str(&format!(
        "Deduplicated records: {}\n",
        email_map.len()
    ));
    log.push_str(&format!(
        "Duplicates removed: {}\n",
        records.len() - email_map.len()
    ));

    // Convert the HashMap back to a Vec, keeping the source sheet names
    let deduplicated: Vec<(StringRecord, String)> = email_map.into_values().collect();

    let records_before = records.len();
    let duplicates = records_before - deduplicated.len();

    DeduplicationResult {
        records: deduplicated,
        log,
        errors,
        records_before_dedup: records_before,
        duplicates_removed: duplicates,
        removed_duplicates,
    }
}

/// Merges two records according to priority rules:
/// 1. Non-empty values overwrite empty values
/// 2. If both non-empty, values from higher priority sheets win (see DEDUPLICATION_PRIORITY)
fn merge_records(
    headers: &StringRecord,
    existing: &StringRecord,
    new: &StringRecord,
    existing_sheet: &str,
    new_sheet: &str,
    log: &mut String,
) -> StringRecord {
    let mut merged = Vec::new();

    for (idx, _header) in headers.iter().enumerate() {
        let existing_val = existing.get(idx).unwrap_or("").trim();
        let new_val = new.get(idx).unwrap_or("").trim();

        let chosen_val = if existing_val.is_empty() && !new_val.is_empty() {
            // Rule 1: Non-empty overwrites empty
            log.push_str(&format!(
                "  Field {}: Using new value '{}' (existing was empty)\n",
                headers.get(idx).unwrap_or("?"),
                new_val
            ));
            new_val
        } else if !existing_val.is_empty() && new_val.is_empty() {
            // Rule 1: Keep non-empty existing value
            existing_val
        } else if existing_val != new_val && !existing_val.is_empty() && !new_val.is_empty() {
            // Rule 2: Both non-empty and different - use priority-based comparison
            if has_higher_priority(new_sheet, existing_sheet) {
                log.push_str(&format!(
                    "  Field {}: Using new value '{}' (sheet '{}' has higher priority than '{}')\n",
                    headers.get(idx).unwrap_or("?"),
                    new_val,
                    new_sheet,
                    existing_sheet
                ));
                new_val
            } else {
                log.push_str(&format!(
                    "  Field {}: Keeping existing value '{}' (sheet '{}' has higher priority than '{}')\n",
                    headers.get(idx).unwrap_or("?"),
                    existing_val,
                    existing_sheet,
                    new_sheet
                ));
                existing_val
            }
        } else {
            // Values are the same or both empty
            existing_val
        };

        merged.push(chosen_val.to_string());
    }

    StringRecord::from(merged)
}

/// Deduplicates records from multiple sheets
/// This version accepts records from different sheets and applies deduplication across all of them
pub fn deduplicate_multi_sheet(
    headers: &StringRecord,
    sheet_records: Vec<(String, Vec<StringRecord>)>,
) -> DeduplicationResult {
    let mut log = String::new();
    let mut errors = Vec::new();
    let mut email_map: HashMap<String, (StringRecord, String)> = HashMap::new();
    let mut removed_duplicates: Vec<(StringRecord, String)> = Vec::new();

    // Count total records before deduplication
    let total_records_before: usize = sheet_records.iter().map(|(_, records)| records.len()).sum();

    // Find the email column index
    let email_idx = headers.iter().position(|h| h == "EMail");

    if email_idx.is_none() {
        errors.push("No 'EMail' column found in headers".to_string());

        // Flatten all records and return them as-is with source sheet names
        let all_records: Vec<(StringRecord, String)> = sheet_records
            .into_iter()
            .flat_map(|(sheet_name, records)| {
                records.into_iter().map(move |r| (r, sheet_name.clone()))
            })
            .collect();

        let records_count = all_records.len();
        return DeduplicationResult {
            records: all_records,
            log,
            errors,
            records_before_dedup: records_count,
            duplicates_removed: 0,
            removed_duplicates: vec![],
        };
    }

    let email_idx = email_idx.unwrap();

    // Find First and Last name indices for fallback key
    let first_idx = headers.iter().position(|h| h == "First");
    let last_idx = headers.iter().position(|h| h == "Last");

    log.push_str("=== MULTI-SHEET DEDUPLICATION LOG ===\n");
    log.push_str("Use CTRL-F to search for specific emails or names\n\n");
    log.push_str("Note: Records with email use email as primary key.\n");
    log.push_str("      Records without email use First+Last name as primary key.\n\n");

    // Process each sheet's records
    for (sheet_name, records) in sheet_records {
        log.push_str(&format!(
            "\n--- Processing sheet: {} ({} records) ---\n",
            sheet_name,
            records.len()
        ));

        for (row_num, record) in records.iter().enumerate() {
            let email = match record.get(email_idx) {
                Some(e) => e.trim().to_uppercase(),
                None => String::new(),
            };

            // Determine the deduplication key: email if present, otherwise First+Last
            let dedup_key = if !email.is_empty() {
                email.clone()
            } else {
                // Use First + Last as fallback key
                let first = first_idx
                    .and_then(|idx| record.get(idx))
                    .unwrap_or("")
                    .trim()
                    .to_uppercase();
                let last = last_idx
                    .and_then(|idx| record.get(idx))
                    .unwrap_or("")
                    .trim()
                    .to_uppercase();

                if first.is_empty() && last.is_empty() {
                    log.push_str(&format!(
                        "  Row {}: Skipping record with no email and no name\n",
                        row_num + 1
                    ));
                    continue;
                }

                format!("NAME:{}|{}", first, last)
            };

            // Check if we've seen this key before
            if let Some((existing_record, existing_sheet)) = email_map.get(&dedup_key) {
                log.push_str(&format!("\n--- DUPLICATE FOUND ---\n"));
                if dedup_key.starts_with("NAME:") {
                    log.push_str(&format!("Name-based key: {}\n", dedup_key.replace("NAME:", "")));
                } else {
                    log.push_str(&format!("Email: {}\n", dedup_key));
                }
                log.push_str(&format!(
                    "  Row {} from sheet '{}'\n",
                    row_num + 1,
                    sheet_name
                ));
                log.push_str(&format!("  Previously seen in sheet '{}'\n", existing_sheet));

                // Store the "loser" in removed duplicates before merging
                if has_higher_priority(&sheet_name, existing_sheet) {
                    // New record has higher priority, existing record is removed
                    removed_duplicates.push((existing_record.clone(), existing_sheet.clone()));
                } else {
                    // Existing record has higher priority, new record is removed
                    removed_duplicates.push((record.clone(), sheet_name.clone()));
                }

                // Merge the records according to priority rules
                let merged = merge_records(
                    headers,
                    existing_record,
                    record,
                    existing_sheet,
                    &sheet_name,
                    &mut log,
                );

                // Determine which sheet name to keep (higher priority sheet)
                let merged_sheet = if has_higher_priority(&sheet_name, existing_sheet) {
                    sheet_name.clone()
                } else {
                    existing_sheet.clone()
                };

                email_map.insert(dedup_key.clone(), (merged, merged_sheet));
            } else {
                if dedup_key.starts_with("NAME:") {
                    log.push_str(&format!(
                        "  Row {}: New name-based record: {}\n",
                        row_num + 1,
                        dedup_key.replace("NAME:", "")
                    ));
                } else {
                    log.push_str(&format!("  Row {}: New email: {}\n", row_num + 1, dedup_key));
                }
                email_map.insert(dedup_key, (record.clone(), sheet_name.clone()));
            }
        }
    }

    log.push_str(&format!("\n=== DEDUPLICATION COMPLETE ===\n"));

    log.push_str(&format!("Unique records found: {}\n", email_map.len()));

    if !errors.is_empty() {
        log.push_str(&format!("Records with errors: {}\n", errors.len()));
    }

    // Convert the HashMap back to a Vec, keeping the source sheet names
    let deduplicated: Vec<(StringRecord, String)> = email_map
        .into_values()
        .collect();

    let duplicates = total_records_before - deduplicated.len();

    DeduplicationResult {
        records: deduplicated,
        log,
        errors,
        records_before_dedup: total_records_before,
        duplicates_removed: duplicates,
        removed_duplicates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplicate_records() {
        let headers = StringRecord::from(vec!["First", "Last", "EMail"]);
        let records = vec![
            StringRecord::from(vec!["John", "Doe", "john@example.com"]),
            StringRecord::from(vec!["Jane", "Doe", "JOHN@EXAMPLE.COM"]),
            StringRecord::from(vec!["Bob", "Smith", "bob@example.com"]),
        ];

        let result = deduplicate_records(&headers, records, "Sheet1");

        assert_eq!(result.records.len(), 2);
        assert!(result.log.contains("DUPLICATE FOUND"));
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_merge_priority_empty_overwrites() {
        let headers = StringRecord::from(vec!["First", "Last", "Phone"]);
        let existing = StringRecord::from(vec!["John", "", "123-456-7890"]);
        let new = StringRecord::from(vec!["John", "Doe", ""]);
        let mut log = String::new();

        let merged = merge_records(&headers, &existing, &new, "Sheet1", "Sheet2", &mut log);

        assert_eq!(merged.get(0).unwrap(), "John");
        assert_eq!(merged.get(1).unwrap(), "Doe"); // Non-empty overwrites empty
        assert_eq!(merged.get(2).unwrap(), "123-456-7890"); // Keep existing non-empty
    }

    #[test]
    fn test_merge_priority_based() {
        let headers = StringRecord::from(vec!["First", "Last"]);
        let existing = StringRecord::from(vec!["John", "Doe"]);
        let new = StringRecord::from(vec!["John", "Smith"]);
        let mut log = String::new();

        // Check has higher priority than Cash according to DEDUPLICATION_PRIORITY
        let merged = merge_records(&headers, &existing, &new, "Cash", "Check", &mut log);

        assert_eq!(merged.get(0).unwrap(), "John");
        assert_eq!(merged.get(1).unwrap(), "Smith"); // Check (new) wins over Cash (existing)
    }

    #[test]
    fn test_deduplicate_by_name_when_no_email() {
        let headers = StringRecord::from(vec!["First", "Last", "EMail", "Phone"]);
        let records = vec![
            StringRecord::from(vec!["John", "Doe", "", "555-1234"]),
            StringRecord::from(vec!["John", "Doe", "", "555-5678"]),
            StringRecord::from(vec!["Jane", "Smith", "jane@example.com", "555-9999"]),
        ];

        let result = deduplicate_records(&headers, records, "Sheet1");

        // Should have 2 unique records: John Doe (merged) and Jane Smith
        assert_eq!(result.records.len(), 2);
        assert!(result.log.contains("Name-based key"));
        assert!(result.log.contains("JOHN|DOE"));
        assert!(result.errors.is_empty());

        // Verify stats
        assert_eq!(result.records_before_dedup, 3);
        assert_eq!(result.duplicates_removed, 1);
    }

    #[test]
    fn test_deduplicate_mixed_email_and_name() {
        let headers = StringRecord::from(vec!["First", "Last", "EMail"]);
        let records = vec![
            StringRecord::from(vec!["John", "Doe", "john@example.com"]),
            StringRecord::from(vec!["John", "Doe", ""]), // Same name, no email
            StringRecord::from(vec!["Jane", "Smith", ""]),
            StringRecord::from(vec!["Jane", "Smith", ""]),
        ];

        let result = deduplicate_records(&headers, records, "Sheet1");

        // Should have 3 unique records:
        // 1. john@example.com (email-based)
        // 2. John Doe with no email (name-based)
        // 3. Jane Smith (name-based, deduplicated from 2 records)
        assert_eq!(result.records.len(), 3);
        assert_eq!(result.duplicates_removed, 1); // Only the duplicate Jane Smith
    }

    #[test]
    fn test_skip_records_with_no_email_and_no_name() {
        let headers = StringRecord::from(vec!["First", "Last", "EMail", "Phone"]);
        let records = vec![
            StringRecord::from(vec!["", "", "", "555-1234"]),
            StringRecord::from(vec!["John", "Doe", "john@example.com", "555-5678"]),
        ];

        let result = deduplicate_records(&headers, records, "Sheet1");

        // Should only have 1 record (the one with email)
        // The record with no email and no name should be skipped
        assert_eq!(result.records.len(), 1);
        assert!(result.log.contains("Skipping record with no email and no name"));
    }
}
