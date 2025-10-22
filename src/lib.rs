// Re-export all mapping-related functionality from data_mappings module
pub mod data_mappings;
pub mod normalization;
pub mod deduplication;

pub use data_mappings::{
    DONORSNAP_FIELDS_WE_CARE_ABOUT,
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
