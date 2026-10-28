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

/// Capitalization normalization for name (and other text) columns

use crate::xlsx_utils::read_all_sheets;
use csv::{ReaderBuilder, Writer};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use utoipa::ToSchema;

/// (sheet_name, headers, rows) - same shape as `read_all_sheets`
pub type SheetData = (String, Vec<String>, Vec<Vec<String>>);

const ROMAN_NUMERAL_SUFFIXES: &[&str] = &["II", "III", "IV", "VI", "VII", "VIII", "IX", "X"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameFixStyle {
    ProperName,
    TitleCase,
    Uppercase,
    Lowercase,
}

impl NameFixStyle {
    pub const ALL: [NameFixStyle; 4] = [
        NameFixStyle::ProperName,
        NameFixStyle::TitleCase,
        NameFixStyle::Uppercase,
        NameFixStyle::Lowercase,
    ];

    pub fn id(&self) -> &'static str {
        match self {
            NameFixStyle::ProperName => "proper_name",
            NameFixStyle::TitleCase => "title_case",
            NameFixStyle::Uppercase => "uppercase",
            NameFixStyle::Lowercase => "lowercase",
        }
    }

    pub fn from_id(id: &str) -> Option<NameFixStyle> {
        Self::ALL.into_iter().find(|s| s.id() == id.trim())
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NameFixStyleInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub example_before: String,
    pub example_after: String,
}

/// Get all available normalization styles
pub fn get_styles() -> Vec<NameFixStyleInfo> {
    let info = |style: NameFixStyle, name: &str, description: &str, before: &str, after: &str| NameFixStyleInfo {
        id: style.id().to_string(),
        name: name.to_string(),
        description: description.to_string(),
        example_before: before.to_string(),
        example_after: after.to_string(),
    };
    vec![
        info(
            NameFixStyle::ProperName,
            "Proper Name",
            "Lowercases everything, then capitalizes the first letter of each word and any letter that follows \
             a non-letter such as a hyphen, apostrophe, period or parenthesis. Also capitalizes the letter after \
             a leading 'Mc' and keeps Roman-numeral suffixes (II-X) fully uppercase.",
            "MARY-KATE O'BRIEN MCDONALD III",
            "Mary-Kate O'Brien McDonald III",
        ),
        info(
            NameFixStyle::TitleCase,
            "Simple Title Case",
            "Capitalizes the first letter of each space-separated word and lowercases the rest. \
             No special handling for hyphens, apostrophes or prefixes.",
            "MARY-KATE O'BRIEN",
            "Mary-kate O'brien",
        ),
        info(
            NameFixStyle::Uppercase,
            "UPPERCASE",
            "Converts every letter to uppercase.",
            "Jane Smith",
            "JANE SMITH",
        ),
        info(
            NameFixStyle::Lowercase,
            "lowercase",
            "Converts every letter to lowercase.",
            "Jane Smith",
            "jane smith",
        ),
    ]
}

/// Normalize the capitalization of a single value.
///
/// When `only_single_case` is true, values containing both uppercase and
/// lowercase letters are returned unchanged. Values with no letters are
/// always returned unchanged.
pub fn fix_value(value: &str, style: NameFixStyle, only_single_case: bool) -> String {
    let has_upper = value.chars().any(|c| c.is_uppercase());
    let has_lower = value.chars().any(|c| c.is_lowercase());
    if !value.chars().any(|c| c.is_alphabetic()) || (only_single_case && has_upper && has_lower) {
        return value.to_string();
    }

    match style {
        NameFixStyle::ProperName => proper_name(value),
        NameFixStyle::TitleCase => map_words(&value.to_lowercase(), capitalize_first),
        NameFixStyle::Uppercase => value.to_uppercase(),
        NameFixStyle::Lowercase => value.to_lowercase(),
    }
}

fn proper_name(value: &str) -> String {
    let mut capitalized = String::with_capacity(value.len());
    let mut prev: Option<char> = None;
    for c in value.to_lowercase().chars() {
        let at_boundary = prev.map_or(true, |p| !p.is_alphanumeric());
        if at_boundary {
            capitalized.extend(c.to_uppercase());
        } else {
            capitalized.push(c);
        }
        prev = Some(c);
    }

    map_words(&capitalized, |word| {
        let bare = word.trim_end_matches(|c| c == '.' || c == ',');
        if ROMAN_NUMERAL_SUFFIXES.contains(&bare.to_uppercase().as_str()) {
            return word.to_uppercase();
        }
        let mut chars = word.chars();
        if let (Some('M'), Some('c'), Some(third)) = (chars.next(), chars.next(), chars.next()) {
            if third.is_alphabetic() {
                return format!("Mc{}{}", third.to_uppercase(), chars.as_str());
            }
        }
        word.to_string()
    })
}

fn capitalize_first(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Apply `f` to each whitespace-separated word, preserving the whitespace itself
fn map_words(s: &str, f: impl Fn(&str) -> String) -> String {
    let mut out = String::with_capacity(s.len());
    let mut word = String::new();
    for c in s.chars() {
        if c.is_whitespace() {
            if !word.is_empty() {
                out.push_str(&f(&word));
                word.clear();
            }
            out.push(c);
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        out.push_str(&f(&word));
    }
    out
}

/// Read an uploaded CSV or Excel file. Format is chosen by file extension.
pub fn read_upload(bytes: &[u8], filename: &str) -> Result<Vec<SheetData>, String> {
    if filename.to_lowercase().ends_with(".csv") {
        return read_csv_bytes(bytes, filename);
    }

    let temp_file = tempfile::NamedTempFile::new()
        .map_err(|e| format!("Failed to create temporary file: {}", e))?;
    std::fs::write(temp_file.path(), bytes).map_err(|e| format!("Failed to save file: {}", e))?;
    read_all_sheets(temp_file.path().to_str().unwrap_or_default()).map_err(|e| {
        format!(
            "Failed to open '{}' as an Excel file: {}\n\n\
            Please upload a valid .csv, .xlsx or .xls file.",
            filename, e
        )
    })
}

/// Parse a UTF-8 CSV (optionally BOM-prefixed) as a single sheet named `sheet_name`
pub fn read_csv_bytes(bytes: &[u8], sheet_name: &str) -> Result<Vec<SheetData>, String> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| {
        format!(
            "'{}' is not a UTF-8 encoded CSV file.\n\n\
            In Excel, use File > Save As > \"CSV UTF-8\", or upload the .xlsx file instead.",
            sheet_name
        )
    })?;

    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut records = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| format!("Failed to read CSV '{}': {}", sheet_name, e))?;
        records.push(record.iter().map(|s| s.to_string()).collect::<Vec<String>>());
    }

    let mut records = records.into_iter();
    match records.next() {
        Some(headers) => {
            let rows: Vec<Vec<String>> = records.collect();
            if rows.is_empty() {
                Ok(vec![])
            } else {
                Ok(vec![(sheet_name.to_string(), headers, rows)])
            }
        }
        None => Ok(vec![]),
    }
}

/// Split a comma-separated column list, trimming and dropping empty entries
pub fn parse_column_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect()
}

fn header_key(header: &str) -> String {
    header.trim().to_lowercase()
}

/// Normalize the selected columns of every sheet and combine the result into
/// a single (headers, rows) table. Multiple sheets get a leading "Source Sheet"
/// column and the union of all sheets' headers.
pub fn fix_sheets(
    sheets: Vec<SheetData>,
    columns: &[String],
    style: NameFixStyle,
    only_single_case: bool,
) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    if columns.is_empty() {
        return Err("Please enter at least one column name to normalize.".to_string());
    }
    if sheets.is_empty() {
        return Err("No data rows found in the uploaded file.".to_string());
    }

    let wanted: HashSet<String> = columns.iter().map(|c| header_key(c)).collect();
    let missing: Vec<&String> = columns
        .iter()
        .filter(|c| !sheets.iter().any(|(_, headers, _)| headers.iter().any(|h| header_key(h) == header_key(c))))
        .collect();
    if !missing.is_empty() {
        let mut available: Vec<&String> = Vec::new();
        for (_, headers, _) in &sheets {
            for h in headers {
                if !available.contains(&h) {
                    available.push(h);
                }
            }
        }
        return Err(format!(
            "Column(s) not found: {}\n\nAvailable columns:\n{}",
            missing.iter().map(|c| format!("'{}'", c)).collect::<Vec<_>>().join(", "),
            available.iter().map(|h| format!("  • {}", h)).collect::<Vec<_>>().join("\n")
        ));
    }

    let multi_sheet = sheets.len() > 1;

    // Union of headers keyed by (normalized header, nth occurrence within its sheet): "Name" and "name "
    // in different sheets share a column, while duplicate names within one sheet stay distinct
    let mut out_headers: Vec<String> = Vec::new();
    let mut union_index: HashMap<(String, usize), usize> = HashMap::new();
    if multi_sheet {
        out_headers.push("Source Sheet".to_string());
    }
    let mut sheet_column_maps: Vec<Vec<usize>> = Vec::new();
    for (_, headers, _) in &sheets {
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut column_map = Vec::with_capacity(headers.len());
        for h in headers {
            let nth = seen.entry(header_key(h)).or_insert(0);
            let key = (header_key(h), *nth);
            *nth += 1;
            let idx = *union_index.entry(key).or_insert_with(|| {
                out_headers.push(h.clone());
                out_headers.len() - 1
            });
            column_map.push(idx);
        }
        sheet_column_maps.push(column_map);
    }

    let mut out_rows = Vec::new();
    for ((sheet_name, headers, rows), column_map) in sheets.into_iter().zip(sheet_column_maps) {
        let selected: Vec<bool> = headers.iter().map(|h| wanted.contains(&header_key(h))).collect();
        for row in rows {
            let mut out_row = vec![String::new(); out_headers.len()];
            if multi_sheet {
                out_row[0] = sheet_name.clone();
            }
            for (col, value) in row.into_iter().enumerate() {
                match column_map.get(col) {
                    Some(&idx) => {
                        out_row[idx] = if selected[col] {
                            fix_value(&value, style, only_single_case)
                        } else {
                            value
                        };
                    }
                    // Cells beyond the header row are kept as-is
                    None => out_row.push(value),
                }
            }
            out_rows.push(out_row);
        }
    }

    Ok((out_headers, out_rows))
}

/// Full pipeline: read upload, normalize selected columns, write CSV.
/// Returns (csv_data, total_data_rows).
pub fn process_name_fixing(
    bytes: &[u8],
    filename: &str,
    columns_input: &str,
    style: NameFixStyle,
    only_single_case: bool,
) -> Result<(String, usize), String> {
    let columns = parse_column_list(columns_input);
    if columns.is_empty() {
        return Err("Please enter at least one column name to normalize.".to_string());
    }
    let sheets = read_upload(bytes, filename)?;
    let (headers, rows) = fix_sheets(sheets, &columns, style, only_single_case)?;
    let total_rows = rows.len();

    let mut csv_output = Vec::new();
    {
        let mut wtr = Writer::from_writer(&mut csv_output);
        wtr.write_record(&headers).map_err(|e| format!("Failed to write CSV row: {}", e))?;
        for row in &rows {
            wtr.write_record(row).map_err(|e| format!("Failed to write CSV row: {}", e))?;
        }
        wtr.flush().map_err(|e| format!("Failed to finalize CSV output: {}", e))?;
    }
    let csv_data = String::from_utf8(csv_output).map_err(|e| format!("Failed to encode CSV as UTF-8: {}", e))?;
    Ok((csv_data, total_rows))
}

/// Download filename for a processed upload: "<sanitized stem>_names_fixed.csv"
pub fn output_filename(uploaded: &str) -> String {
    let stem = std::path::Path::new(uploaded)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let safe: String = stem
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '.' | '-'))
        .collect();
    let safe = safe.trim();
    format!("{}_names_fixed.csv", if safe.is_empty() { "names" } else { safe })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xlsx_utils::write_xlsx_to_bytes;

    fn proper(v: &str) -> String {
        fix_value(v, NameFixStyle::ProperName, true)
    }

    #[test]
    fn test_proper_name_all_caps() {
        assert_eq!(proper("JANE SMITH"), "Jane Smith");
    }

    #[test]
    fn test_proper_name_apostrophe_hyphen_mc() {
        assert_eq!(proper("O'BRIEN"), "O'Brien");
        assert_eq!(proper("mary-kate smith-jones"), "Mary-Kate Smith-Jones");
        assert_eq!(proper("MCDONALD"), "McDonald");
        assert_eq!(proper("JACK (JACKIE) SMITH"), "Jack (Jackie) Smith");
    }

    #[test]
    fn test_proper_name_roman_numerals() {
        assert_eq!(proper("JOHN SMITH III"), "John Smith III");
        assert_eq!(proper("john smith iv."), "John Smith IV.");
    }

    #[test]
    fn test_title_case() {
        assert_eq!(fix_value("O'BRIEN SMITH-JONES", NameFixStyle::TitleCase, true), "O'brien Smith-jones");
    }

    #[test]
    fn test_upper_and_lower() {
        assert_eq!(fix_value("Jane Smith", NameFixStyle::Uppercase, false), "JANE SMITH");
        assert_eq!(fix_value("Jane Smith", NameFixStyle::Lowercase, false), "jane smith");
    }

    #[test]
    fn test_only_single_case_option() {
        assert_eq!(proper("DeAndre MacKenzie"), "DeAndre MacKenzie");
        assert_eq!(proper("DEANDRE MACKENZIE"), "Deandre Mackenzie");
        assert_eq!(fix_value("jANE sMITH", NameFixStyle::ProperName, false), "Jane Smith");
    }

    #[test]
    fn test_preserves_non_letter_content() {
        assert_eq!(proper(""), "");
        assert_eq!(proper("12345"), "12345");
        assert_eq!(proper("  JANE   SMITH "), "  Jane   Smith ");
        assert_eq!(proper("JOSÉ PEÑA"), "José Peña");
    }

    #[test]
    fn test_style_examples_round_trip() {
        for info in get_styles() {
            let style = NameFixStyle::from_id(&info.id).unwrap();
            assert_eq!(fix_value(&info.example_before, style, false), info.example_after, "style {}", info.id);
        }
    }

    #[test]
    fn test_csv_bom_stripped() {
        let sheets = read_csv_bytes(b"\xEF\xBB\xBFName,Email\nJANE SMITH,j@x.org\n", "t.csv").unwrap();
        assert_eq!(sheets[0].1, vec!["Name", "Email"]);
    }

    #[test]
    fn test_csv_non_utf8_error() {
        let err = read_csv_bytes(b"Name\nJOS\xC9\n", "t.csv").unwrap_err();
        assert!(err.contains("not a UTF-8"));
    }

    #[test]
    fn test_multiple_columns_case_insensitive() {
        let csv = b"First Name,Last Name,Email\nJANE,SMITH,JANE@X.ORG\n";
        let (out, rows) = process_name_fixing(csv, "d.csv", "First Name, last name", NameFixStyle::ProperName, true).unwrap();
        assert_eq!(rows, 1);
        assert_eq!(out, "First Name,Last Name,Email\nJane,Smith,JANE@X.ORG\n");
    }

    #[test]
    fn test_missing_column_error() {
        let err = process_name_fixing(b"Name\nJANE\n", "d.csv", "Full Name", NameFixStyle::ProperName, true).unwrap_err();
        assert!(err.contains("'Full Name'"));
        assert!(err.contains("• Name"));
    }

    #[test]
    fn test_empty_column_input_error() {
        let err = process_name_fixing(b"Name\nJANE\n", "d.csv", " , ", NameFixStyle::ProperName, true).unwrap_err();
        assert!(err.contains("at least one column"));
    }

    #[test]
    fn test_multi_sheet_union() {
        let xlsx = write_xlsx_to_bytes(vec![
            ("Adopters".to_string(), vec!["Name".into(), "Phone".into()], vec![vec!["JANE SMITH".into(), "555".into()]]),
            ("Fosters".to_string(), vec!["name ".into(), "City".into()], vec![vec!["bob jones".into(), "ROCKFORD".into()]]),
        ])
        .unwrap();
        let (out, rows) = process_name_fixing(&xlsx, "lists.xlsx", "name", NameFixStyle::ProperName, true).unwrap();
        assert_eq!(rows, 2);
        assert_eq!(
            out,
            "Source Sheet,Name,Phone,City\nAdopters,Jane Smith,555,\nFosters,Bob Jones,,ROCKFORD\n"
        );
    }

    #[test]
    fn test_output_filename() {
        assert_eq!(output_filename("donors.xlsx"), "donors_names_fixed.csv");
        assert_eq!(output_filename("we\"ird;name.csv"), "weirdname_names_fixed.csv");
        assert_eq!(output_filename("\".csv"), "names_names_fixed.csv");
    }
}
