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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

// Fake data generation for sample spreadsheet display
const FAKE_FIRST_NAMES: &[&str] = &[
    "Jane", "Robert", "Emily", "Michael", "Sarah", "David", "Lisa", "Thomas", "Karen", "James",
];
const FAKE_LAST_NAMES: &[&str] = &[
    "Smith", "Johnson", "Davis", "Brown", "Williams", "Martinez", "Anderson", "Garcia", "Wilson",
    "Taylor",
];
const FAKE_EMAILS: &[&str] = &[
    "jsmith@example.com",
    "rjohnson@example.com",
    "edavis@example.com",
    "mbrown@example.com",
    "swilliams@example.com",
    "dmartinez@example.com",
    "landerson@example.com",
    "tgarcia@example.com",
    "kwilson@example.com",
    "jtaylor@example.com",
];
const FAKE_PHONES: &[&str] = &[
    "555-0100", "555-0101", "555-0102", "555-0103", "555-0104", "555-0105", "555-0106", "555-0107",
    "555-0108", "555-0109",
];
const FAKE_ADDRESSES: &[&str] = &[
    "123 Main St",
    "456 Oak Ave",
    "789 Elm Street",
    "321 Pine Rd",
    "654 Maple Dr",
    "987 Cedar Ln",
    "159 Birch Ave",
    "753 Willow St",
    "246 Spruce Ct",
    "802 Ash Blvd",
];
const FAKE_CITIES: &[&str] = &[
    "Springfield",
    "Chicago",
    "Naperville",
    "Evanston",
    "Aurora",
    "Schaumburg",
    "Joliet",
    "Rockford",
    "Deerfield",
    "Wheaton",
];
const FAKE_ZIPS: &[&str] = &[
    "62701", "60601", "60540", "60201", "60505", "60193", "60435", "61101", "60015", "60187",
];
const FAKE_AMOUNTS: &[&str] = &["50.00", "100.00", "75.00", "150.00", "200.00", "125.00"];
const FAKE_DATES: &[&str] = &[
    "01/15/2025",
    "01/16/2025",
    "02/10/2025",
    "02/11/2025",
    "02/15/2025",
    "02/16/2025",
    "02/20/2025",
    "02/21/2025",
];
const FAKE_NOTES: &[&str] = &[
    "Annual donation",
    "In memory of Fluffy",
    "Pet Food Pantry",
    "Adoption fee",
    "General fund",
    "Memorial donation",
];

pub const DONORSNAP_FIELDS_WE_CARE_ABOUT: &[&str] = &[
    "First",
    "Last",
    "Company",
    "EMail",
    "Phone",
    "Address1",
    "Address2",
    "Address3",
    "City",
    "State/Province",
    "Zip/Postal Code",
    "Country",
    "Salutation",
    "Donation Date",
    "Amount",
    "Donation Type",
    "Payment Method",
    "DonationNote",
];

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FieldDescription {
    pub target_field: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetMapping {
    pub sheet_name: &'static str,
    pub mappings: Vec<(&'static str, &'static str)>, // (target_field, source_field)
}

impl SheetMapping {
    pub fn to_hashmap(&self) -> HashMap<&'static str, &'static str> {
        self.mappings.iter().cloned().collect()
    }
}

pub fn get_field_descriptions() -> Vec<FieldDescription> {
    vec![
        FieldDescription {
            target_field: "First",
            description: "First name",
        },
        FieldDescription {
            target_field: "Last",
            description: "Last name",
        },
        FieldDescription {
            target_field: "Company",
            description: "Company (if applicable)",
        },
        FieldDescription {
            target_field: "EMail",
            description: "Email address",
        },
        FieldDescription {
            target_field: "Phone",
            description: "Phone number",
        },
        FieldDescription {
            target_field: "Address1",
            description: "Address line 1",
        },
        FieldDescription {
            target_field: "Address2",
            description: "Address line 2",
        },
        FieldDescription {
            target_field: "Address3",
            description: "Address line 3",
        },
        FieldDescription {
            target_field: "City",
            description: "City",
        },
        FieldDescription {
            target_field: "State/Province",
            description: "State or Province",
        },
        FieldDescription {
            target_field: "Zip/Postal Code",
            description: "Zip or Postal Code",
        },
        FieldDescription {
            target_field: "Country",
            description: "Country",
        },
        FieldDescription {
            target_field: "Salutation",
            description: "Salutation",
        },
        FieldDescription {
            target_field: "Donation Date",
            description: "Date of donation",
        },
        FieldDescription {
            target_field: "Amount",
            description: "Donation amount",
        },
        FieldDescription {
            target_field: "Donation Type",
            description: "Type of donation",
        },
        FieldDescription {
            target_field: "Payment Method",
            description: "Payment method used",
        },
        FieldDescription {
            target_field: "DonationNote",
            description: "Notes about the donation",
        },
    ]
}

pub fn get_all_sheet_mappings() -> Vec<SheetMapping> {
    vec![
        // DonorSnap - exact field names
        SheetMapping {
            sheet_name: "DonorSnap",
            mappings: vec![
                ("First", "First"),
                ("Last", "Last"),
                ("Company", "Company"),
                ("EMail", "EMail"),
                ("Phone", "Phone"),
                ("Address1", "Address1"),
                ("Address2", "Address2"),
                ("Address3", "Address3"),
                ("City", "City"),
                ("State/Province", "State/Province"),
                ("Zip/Postal Code", "Zip/Postal Code"),
                ("Country", "Country"),
                ("Salutation", "Salutation"),
                ("Donation Date", "Donation Date"),
                ("Amount", "Amount"),
                ("Donation Type", "Donation Type"),
                ("Payment Method", "Payment Method"),
                ("DonationNote", "DonationNote"),
                ("Species", "Species"),
                ("Appeal", "Appeal"),
            ],
        },
        // Qgiv mappings
        SheetMapping {
            sheet_name: "Qgiv",
            mappings: vec![
                ("First", "First Name"),
                ("Last", "Last Name"),
                ("Company", "Company"),
                ("EMail", "Email"),
                ("Phone", "Phone"),
                ("Address1", "Address"),
                // ("Address2", "Billing Address"), //removed 10-20-2025 for Holly
                ("City", "City"),
                ("State/Province", "State"),
                ("Zip/Postal Code", "Zip"),
                ("Country", "Country"),
                ("Salutation", "Salutation"),
                ("Donation Date", "Date"),
                ("Amount", "Total Amount"),
                ("Donation Type", "Type"),
                ("Payment Method", "Payment Method"),
                ("DonationNote", "Notes"),
                ("Appeal", "Appeal"),
            ],
        },
        // ShelterLuv mappings
        SheetMapping {
            sheet_name: "ShelterLuv",
            mappings: vec![
                ("First", "Person First Name"),
                ("Last", "Person Last Name"),
                ("EMail", "Person Primary Email"),
                ("Phone", "Person Primary Phone"),
                ("Address1", "Person Street Address 1"),
                ("Address2", "Person Street Address 2"),
                ("City", "Person City"),
                ("State/Province", "Person State"),
                ("Zip/Postal Code", "Person Zip"),
                ("Donation Date", "Donation Date"),
                ("Amount", "Total Donation"),
                ("DonationNote", "Transaction Memo"),
                ("Appeal", "Appeal"),
            ],
        },
        // Square mappings
        SheetMapping {
            sheet_name: "Square",
            mappings: vec![
                ("First", "First Name"),
                ("Last", "Last Name"),
                ("Company", "Company Name"),
                ("EMail", "Email Address"),
                ("Phone", "Phone Number"),
                ("Donation Date", "Date"),
                ("Address1", "Street Address 1"),
                ("Address2", "Street Address 2"),
                ("City", "City"),
                ("State/Province", "State"),
                ("Zip/Postal Code", "Postal Code"),
                ("Amount", "Total Spend"),
                ("DonationNote", "Memo"),
                ("Appeal", "Appeal"),
            ],
        },
        // Facebook PayPal mappings
        SheetMapping {
            sheet_name: "Facebook PayPal",
            mappings: vec![
                ("Last", "First Name"),
                ("First", "Last Name"),
                ("EMail", "From Email Address"),
                ("Address1", "Address Line 1"),
                ("Address2", "Address Line 2/District/Neighborhood"),
                ("City", "Town/City"),
                (
                    "State/Province",
                    "State/Province/Region/County/Territory/Prefecture/Republic",
                ),
                ("Zip/Postal Code", "Zip/Postal Code"),
                ("Donation Date", "Date"),
                ("Amount", "Gross"),
                ("Donation Type", "Type"),
                ("DonationNote", "Note"),
                ("Appeal", "Appeal"),
            ],
        },
        // Benevity.org mappings (matching gifts)
        SheetMapping {
            sheet_name: "Benevity.org",
            mappings: vec![
                ("First", "Donor First Name"),
                ("Last", "Donor Last Name"),
                ("Company", "Company"),
                ("EMail", "Email"),
                ("Zip/Postal Code", "Postal Code"),
                ("Donation Date", "Donation Date"),
                ("Amount", "Total Donation to be Acknowledged"),
                ("DonationNote", "Activity/Comment"),
                ("Appeal", "Appeal"),
            ],
        },
        // CARE Volunteer List mappings
        SheetMapping {
            sheet_name: "CARE Volunteer List",
            mappings: vec![
                ("First", "First Name"),
                ("Last", "Last Name"),
                ("EMail", "Email"),
                ("Phone", "Phone"),
                ("Address1", "Mailing Street"),
                ("City", "Mailing City"),
                ("State/Province", "Mailing State/Province"),
                ("Zip/Postal Code", "Mailing Zip/Postal Code"),
                ("Appeal", "Appeal"),
                // Note: The following fields from the volunteer list don't map to standard DonorSnap fields:
                // - Volunteer Status
                // - Preferred Pronouns
                // - Volunteer Roles
                // These could be added as custom fields in the future if needed
            ],
        },
        // Check donations mappings
        SheetMapping {
            sheet_name: "Check",
            mappings: vec![
                ("First", "First Name"),
                ("Last", "Last Name"),
                ("Address1", "Address"),
                ("City", "City"),
                ("State/Province", "State"),
                ("Zip/Postal Code", "Zip"),
                ("EMail", "Email"),
                ("Donation Date", "Date"),
                ("Amount", "Amount"),
                ("Payment Method", "Payment Method"),
                ("DonationNote", "Notes"),
            ],
        },
        // Cash donations mappings
        SheetMapping {
            sheet_name: "Cash",
            mappings: vec![
                ("First", "First Name"),
                ("Last", "Last Name"),
                ("Address1", "Address"),
                ("City", "City"),
                ("State/Province", "State"),
                ("Zip/Postal Code", "Zip"),
                ("EMail", "Email"),
                ("Donation Date", "Date"),
                ("Amount", "Amount"),
                ("Payment Method", "Payment Method"),
                ("DonationNote", "Notes"),
            ],
        },
    ]
}

// Generate fake sample data for a given source field
pub fn generate_fake_data(source_field: &str, row_index: usize) -> String {
    let idx = row_index % 10;

    match source_field {
        // Names
        "First Name" | "First" | "Person First Name" | "Donor First Name" => {
            FAKE_FIRST_NAMES[idx].to_string()
        }
        "Last Name" | "Last" | "Person Last Name" | "Donor Last Name" => {
            FAKE_LAST_NAMES[idx].to_string()
        }
        "Company" | "Company Name" => if idx % 3 == 0 { "" } else { "Example Corp" }.to_string(),

        // Contact
        "Email" | "EMail" | "Email Address" | "Person Primary Email" | "From Email Address" => {
            FAKE_EMAILS[idx].to_string()
        }
        "Phone" | "Phone Number" | "Person Primary Phone" => format!("(312) {}", FAKE_PHONES[idx]),

        // Address
        "Address"
        | "Address1"
        | "Street Address 1"
        | "Person Street Address 1"
        | "Address Line 1" => FAKE_ADDRESSES[idx].to_string(),
        "Billing Address"
        | "Address2"
        | "Street Address 2"
        | "Person Street Address 2"
        | "Address Line 2/District/Neighborhood" => "".to_string(),
        "City" | "Person City" | "Town/City" => FAKE_CITIES[idx].to_string(),
        "State"
        | "State/Province"
        | "Person State"
        | "State/Province/Region/County/Territory/Prefecture/Republic" => "IL".to_string(),
        "Zip" | "Zip/Postal Code" | "Postal Code" | "Person Zip" => FAKE_ZIPS[idx].to_string(),
        "Country" => "US".to_string(),

        // Donation info
        "Date" | "Donation Date" | "Tran Date" => FAKE_DATES[idx % FAKE_DATES.len()].to_string(),
        "Total Amount"
        | "Amount"
        | "Gross"
        | "Total Paid (Gross)"
        | "Total Spend"
        | "Total Donation to be Acknowledged" => FAKE_AMOUNTS[idx % FAKE_AMOUNTS.len()].to_string(),
        "Type" | "Donation Type" => "one-time".to_string(),
        "Payment Method" => if idx % 2 == 0 { "Credit Card" } else { "Check" }.to_string(),
        "Notes" | "DonationNote" | "Memo" | "Transaction Memo" | "Activity/Comment" => {
            FAKE_NOTES[idx % FAKE_NOTES.len()].to_string()
        }
        "Salutation" => FAKE_FIRST_NAMES[idx].to_string(),

        // Other common fields
        "Status" => "Accepted".to_string(),
        "Time (ET)" | "Tran Time" => format!("{}:30", 9 + (idx % 8)),

        // CARE Volunteer List specific fields
        "Mailing Street" => FAKE_ADDRESSES[idx].to_string(),
        "Mailing City" => FAKE_CITIES[idx].to_string(),
        "Mailing State/Province" => "IL".to_string(),
        "Mailing Zip/Postal Code" => FAKE_ZIPS[idx].to_string(),
        "Volunteer Status" => if idx % 3 == 0 { "Active" } else if idx % 3 == 1 { "Inactive" } else { "Pending" }.to_string(),
        "Preferred Pronouns" => ["She/Her", "He/Him", "They/Them"][idx % 3].to_string(),
        "Volunteer Roles" => ["Dog Shift Volunteer", "Cat Care", "Event Volunteer", "Foster"][idx % 4].to_string(),
        "Appeal" => ["Cat", "Dog", "Both", "General"][idx % 4].to_string(),

        _ => "".to_string(),
    }
}

// Generate sample rows for a sheet
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SampleRow {
    pub cells: Vec<String>,
}

pub fn generate_sample_data_for_sheet(
    sheet_mapping: &SheetMapping,
    num_rows: usize,
) -> (Vec<String>, Vec<SampleRow>) {
    // Get all source field names (headers)
    let headers: Vec<String> = sheet_mapping
        .mappings
        .iter()
        .map(|(_, source)| source.to_string())
        .collect();

    // Generate sample rows
    let mut rows = Vec::new();
    for row_idx in 0..num_rows {
        let cells: Vec<String> = sheet_mapping
            .mappings
            .iter()
            .map(|(_, source)| generate_fake_data(source, row_idx))
            .collect();
        rows.push(SampleRow { cells });
    }

    (headers, rows)
}
