/// Deduplication logic for merging records based on email addresses

use std::collections::HashMap;
use csv::StringRecord;

/// Represents the result of a deduplication operation
pub struct DeduplicationResult {
    /// The deduplicated records
    pub records: Vec<StringRecord>,
    /// Verbose log of all deduplication operations
    pub log: String,
    /// Any errors encountered during deduplication
    pub errors: Vec<String>,
    /// Number of records before deduplication
    pub records_before_dedup: usize,
    /// Number of duplicates removed
    pub duplicates_removed: usize,
}

/// Deduplicates records based on email addresses (case-insensitive)
///
/// Priority rules:
/// 1. Non-empty values overwrite empty values
/// 2. If both non-empty, values with source sheet names alphabetically closer to "A" win
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

    // Find the email column index
    let email_idx = headers.iter().position(|h| h == "EMail");

    if email_idx.is_none() {
        errors.push("No 'EMail' column found in headers".to_string());
        let records_count = records.len();
        return DeduplicationResult {
            records,
            log,
            errors,
            records_before_dedup: records_count,
            duplicates_removed: 0,
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

            // Merge the records according to priority rules
            let merged = merge_records(
                headers,
                existing_record,
                record,
                existing_sheet,
                source_sheet_name,
                &mut log,
            );

            // Determine which sheet name to keep
            let merged_sheet = if source_sheet_name < existing_sheet {
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

    // Convert the HashMap back to a Vec
    let deduplicated: Vec<StringRecord> = email_map.into_values().map(|(record, _)| record).collect();

    let records_before = records.len();
    let duplicates = records_before - deduplicated.len();

    DeduplicationResult {
        records: deduplicated,
        log,
        errors,
        records_before_dedup: records_before,
        duplicates_removed: duplicates,
    }
}

/// Merges two records according to priority rules:
/// 1. Non-empty values overwrite empty values
/// 2. If both non-empty, values from sheets alphabetically closer to "A" win
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
            // Rule 2: Both non-empty and different - use alphabetical priority
            if new_sheet < existing_sheet {
                log.push_str(&format!(
                    "  Field {}: Using new value '{}' (sheet '{}' < '{}')\n",
                    headers.get(idx).unwrap_or("?"),
                    new_val,
                    new_sheet,
                    existing_sheet
                ));
                new_val
            } else {
                log.push_str(&format!(
                    "  Field {}: Keeping existing value '{}' (sheet '{}' >= '{}')\n",
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

    // Count total records before deduplication
    let total_records_before: usize = sheet_records.iter().map(|(_, records)| records.len()).sum();

    // Find the email column index
    let email_idx = headers.iter().position(|h| h == "EMail");

    if email_idx.is_none() {
        errors.push("No 'EMail' column found in headers".to_string());

        // Flatten all records and return them as-is
        let all_records: Vec<StringRecord> = sheet_records
            .into_iter()
            .flat_map(|(_, records)| records)
            .collect();

        let records_count = all_records.len();
        return DeduplicationResult {
            records: all_records,
            log,
            errors,
            records_before_dedup: records_count,
            duplicates_removed: 0,
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

                // Merge the records according to priority rules
                let merged = merge_records(
                    headers,
                    existing_record,
                    record,
                    existing_sheet,
                    &sheet_name,
                    &mut log,
                );

                // Determine which sheet name to keep (alphabetically first)
                let merged_sheet = if sheet_name < *existing_sheet {
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

    // Convert the HashMap back to a Vec
    let deduplicated: Vec<StringRecord> = email_map
        .into_values()
        .map(|(record, _)| record)
        .collect();

    let duplicates = total_records_before - deduplicated.len();

    DeduplicationResult {
        records: deduplicated,
        log,
        errors,
        records_before_dedup: total_records_before,
        duplicates_removed: duplicates,
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
    fn test_merge_priority_alphabetical() {
        let headers = StringRecord::from(vec!["First", "Last"]);
        let existing = StringRecord::from(vec!["John", "Doe"]);
        let new = StringRecord::from(vec!["John", "Smith"]);
        let mut log = String::new();

        // Sheet A comes before Sheet B alphabetically
        let merged = merge_records(&headers, &existing, &new, "SheetB", "SheetA", &mut log);

        assert_eq!(merged.get(0).unwrap(), "John");
        assert_eq!(merged.get(1).unwrap(), "Smith"); // SheetA wins
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
