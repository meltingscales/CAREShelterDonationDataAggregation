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

// Re-export all mapping-related functionality from data_mappings module
pub mod data_mappings;
pub mod normalization;
pub mod deduplication;
pub mod name_splitter;
pub mod xlsx_utils;

pub use data_mappings::{
    DONORSNAP_FIELDS_WE_CARE_ABOUT,
    DEDUPLICATION_PRIORITY,
    FieldDescription,
    SheetMapping,
    SampleRow,
    get_field_descriptions,
    get_all_sheet_mappings,
    generate_fake_data,
    generate_sample_data_for_sheet,
};

pub use normalization::{normalize_phone, normalize_state};
pub use deduplication::{deduplicate_records, deduplicate_multi_sheet, DeduplicationResult};
pub use name_splitter::{
    get_algorithms,
    apply_all_algorithms,
    NameSplitResult,
    NameSplitAlgorithm,
};
pub use xlsx_utils::{
    read_all_sheets,
    write_xlsx_to_bytes,
    deduplicate_sheet_rows,
    data_to_string,
};
