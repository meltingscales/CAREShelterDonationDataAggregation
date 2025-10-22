use calamine::{open_workbook, Reader, Xls};
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "./C.A.R.E. Donation Spreadsheet.xls";
    let mut workbook: Xls<_> = open_workbook(path)?;

    let sheet = workbook.worksheet_range("DonorSnap")?;

    if let Some(first_row) = sheet.rows().next() {
        let field_names: Vec<String> = first_row.iter().map(|cell| cell.to_string()).collect();

        println!("Field names: {:?}", field_names);

        let json = serde_json::to_string_pretty(&field_names)?;
        let mut file = File::create("donorsnap_field_names.json")?;
        file.write_all(json.as_bytes())?;

        println!("Field names saved to donorsnap_field_names.json");
    }

    Ok(())
}
