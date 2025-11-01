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

use calamine::{open_workbook_auto, Data, Reader};
use rust_xlsxwriter::{Workbook, Worksheet, Format};
use std::collections::HashMap;

/// Convert calamine Data to String
pub fn data_to_string(data: &Data) -> String {
    match data {
        Data::String(s) => s.clone(),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Bool(b) => b.to_string(),
        Data::Error(_) => String::new(),
        Data::Empty => String::new(),
    }
}

/// Read all sheets from an XLSX file and return as structured data
/// Returns: Vec<(sheet_name, headers, rows)>
pub fn read_all_sheets(file_path: &str) -> Result<Vec<(String, Vec<String>, Vec<Vec<String>>)>, String> {
    let mut workbook = open_workbook_auto(file_path)
        .map_err(|e| format!("Failed to open workbook: {}", e))?;

    let sheet_names = workbook.sheet_names().to_vec();
    let mut result = Vec::new();

    for sheet_name in &sheet_names {
        let sheet = match workbook.worksheet_range(sheet_name) {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to read sheet '{}': {}", sheet_name, e)),
        };

        let mut rows_iter = sheet.rows();

        // Get header row
        let headers = match rows_iter.next() {
            Some(row) => row.iter().map(|cell| data_to_string(cell)).collect::<Vec<String>>(),
            None => continue, // Skip empty sheets
        };

        // Get data rows
        let mut data_rows = Vec::new();
        for row in rows_iter {
            let row_data: Vec<String> = row.iter().map(|cell| data_to_string(cell)).collect();
            data_rows.push(row_data);
        }

        if !data_rows.is_empty() {
            result.push((sheet_name.clone(), headers, data_rows));
        }
    }

    Ok(result)
}

/// Write data to XLSX file in memory and return as bytes
pub fn write_xlsx_to_bytes(
    sheets_data: Vec<(String, Vec<String>, Vec<Vec<String>>)>,
) -> Result<Vec<u8>, String> {
    let mut workbook = Workbook::new();

    // Create header format
    let header_format = Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::RGB(0xD3D3D3));

    for (sheet_name, headers, rows) in sheets_data {
        let mut worksheet = Worksheet::new();

        // Write headers
        for (col_idx, header) in headers.iter().enumerate() {
            worksheet
                .write_string_with_format(0, col_idx as u16, header, &header_format)
                .map_err(|e| format!("Failed to write header: {}", e))?;
        }

        // Write data rows
        for (row_idx, row_data) in rows.iter().enumerate() {
            for (col_idx, cell_value) in row_data.iter().enumerate() {
                worksheet
                    .write_string((row_idx + 1) as u32, col_idx as u16, cell_value)
                    .map_err(|e| format!("Failed to write cell: {}", e))?;
            }
        }

        // Auto-fit columns
        worksheet.autofit();

        // Set sheet name before adding
        worksheet
            .set_name(&sheet_name)
            .map_err(|e| format!("Failed to set worksheet name '{}': {}", sheet_name, e))?;

        // Add worksheet to workbook
        workbook.push_worksheet(worksheet);
    }

    // Write to buffer
    let buffer = workbook
        .save_to_buffer()
        .map_err(|e| format!("Failed to save workbook to buffer: {}", e))?;

    Ok(buffer)
}

/// Deduplicate rows within a sheet based on email and name
/// Returns: (deduplicated_rows, deduplication_log, records_before, duplicates_removed)
pub fn deduplicate_sheet_rows(
    headers: &[String],
    rows: Vec<Vec<String>>,
) -> (Vec<Vec<String>>, String, usize, usize) {
    let mut log = String::new();
    let mut dedup_map: HashMap<String, Vec<String>> = HashMap::new();

    // Find email, first name, and last name column indices
    let email_idx = headers.iter().position(|h| h.eq_ignore_ascii_case("email") || h.eq_ignore_ascii_case("EMail"));
    let first_idx = headers.iter().position(|h| h.eq_ignore_ascii_case("first") || h.eq_ignore_ascii_case("First Name"));
    let last_idx = headers.iter().position(|h| h.eq_ignore_ascii_case("last") || h.eq_ignore_ascii_case("Last Name"));

    let records_before = rows.len();
    log.push_str(&format!("Processing {} rows\n", records_before));
    log.push_str("Note: Deduplication uses Email as primary key, First+Last name as fallback\n\n");

    for (row_num, row) in rows.iter().enumerate() {
        // Determine deduplication key
        let dedup_key = if let Some(email_idx) = email_idx {
            let email = row.get(email_idx).unwrap_or(&String::new()).trim().to_uppercase();
            if !email.is_empty() {
                email
            } else {
                // Fallback to name-based key
                let first = first_idx
                    .and_then(|idx| row.get(idx))
                    .unwrap_or(&String::new())
                    .trim()
                    .to_uppercase();
                let last = last_idx
                    .and_then(|idx| row.get(idx))
                    .unwrap_or(&String::new())
                    .trim()
                    .to_uppercase();

                if first.is_empty() && last.is_empty() {
                    log.push_str(&format!("Row {}: Skipping - no email and no name\n", row_num + 1));
                    continue;
                }

                format!("NAME:{}|{}", first, last)
            }
        } else {
            // No email column, use name-based key
            let first = first_idx
                .and_then(|idx| row.get(idx))
                .unwrap_or(&String::new())
                .trim()
                .to_uppercase();
            let last = last_idx
                .and_then(|idx| row.get(idx))
                .unwrap_or(&String::new())
                .trim()
                .to_uppercase();

            if first.is_empty() && last.is_empty() {
                log.push_str(&format!("Row {}: Skipping - no name\n", row_num + 1));
                continue;
            }

            format!("NAME:{}|{}", first, last)
        };

        if dedup_map.contains_key(&dedup_key) {
            log.push_str(&format!(
                "Row {}: DUPLICATE - Key: {} (keeping first occurrence)\n",
                row_num + 1,
                dedup_key
            ));
        } else {
            dedup_map.insert(dedup_key, row.clone());
        }
    }

    let deduplicated_rows: Vec<Vec<String>> = dedup_map.into_values().collect();
    let duplicates_removed = records_before - deduplicated_rows.len();

    log.push_str(&format!(
        "\n=== DEDUPLICATION COMPLETE ===\n\
         Original rows: {}\n\
         Unique rows: {}\n\
         Duplicates removed: {}\n",
        records_before,
        deduplicated_rows.len(),
        duplicates_removed
    ));

    (deduplicated_rows, log, records_before, duplicates_removed)
}
