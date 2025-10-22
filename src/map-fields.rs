use calamine::{open_workbook, Reader, Xls, Data};
use care_shelter_donation_aggregation::{get_all_sheet_mappings, DONORSNAP_FIELDS_WE_CARE_ABOUT};
use csv::Writer;
use std::collections::HashMap;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut wtr = Writer::from_path("output.csv")?;
    wtr.write_record(DONORSNAP_FIELDS_WE_CARE_ABOUT)?;

    // Process all sheets using the shared mappings
    let mappings = get_all_sheet_mappings();
    for sheet_mapping in mappings {
        match extract_sheet_data(&mut wtr, "C.A.R.E. Donation Spreadsheet.xls", &sheet_mapping) {
            Ok(_) => println!("{} data processed", sheet_mapping.sheet_name),
            Err(e) => eprintln!("Warning: Could not process {}: {}", sheet_mapping.sheet_name, e),
        }
    }

    wtr.flush()?;
    println!("All data extracted to output.csv");
    Ok(())
}

fn data_to_string(data: &Data) -> String {
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

fn extract_sheet_data<W: std::io::Write>(
    wtr: &mut Writer<W>,
    file_path: &str,
    sheet_mapping: &care_shelter_donation_aggregation::SheetMapping,
) -> Result<(), Box<dyn Error>> {
    let mut workbook: Xls<_> = open_workbook(file_path)?;

    let sheet = workbook.worksheet_range(sheet_mapping.sheet_name)?;
    let mut rows = sheet.rows();

    // Get header row to map column indices
    let headers = match rows.next() {
        Some(row) => row.iter().map(|cell| data_to_string(cell)).collect::<Vec<String>>(),
        None => return Err(format!("No header row found in {}", sheet_mapping.sheet_name).into()),
    };

    // Create mapping of target field names to column indices
    let field_mappings = sheet_mapping.to_hashmap();
    let mut field_indices = HashMap::new();
    for (&target_field, &source_field) in &field_mappings {
        if let Some(index) = headers.iter().position(|h| h == source_field) {
            field_indices.insert(target_field, index);
        }
    }

    // Process data rows
    for row in rows {
        let mut record = Vec::new();
        for field in DONORSNAP_FIELDS_WE_CARE_ABOUT {
            let value = match field_indices.get(field) {
                Some(&index) => {
                    if index < row.len() {
                        data_to_string(&row[index])
                    } else {
                        String::new()
                    }
                }
                None => String::new(),
            };
            record.push(value);
        }
        wtr.write_record(&record)?;
    }

    Ok(())
}

