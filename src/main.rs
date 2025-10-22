use askama::Template;
use axum::{
    extract::{Multipart, Path, State},
    response::{IntoResponse, Response, Json},
    routing::{get, post},
    Router,
};
use calamine::{open_workbook_auto, Data, Reader};
use care_shelter_donation_aggregation::{
    get_all_sheet_mappings, get_field_descriptions, DONORSNAP_FIELDS_WE_CARE_ABOUT,
    normalize_phone, normalize_state, deduplicate_multi_sheet,
};
use csv::{Writer, StringRecord};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;
use serde::Serialize;
use uuid::Uuid;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    sheet_names: Vec<String>,
}

#[derive(Template)]
#[template(path = "mappings.html")]
struct MappingsTemplate {}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    error_message: String,
}

#[derive(Template)]
#[template(path = "faq.html")]
struct FaqTemplate {
    download_expiry_seconds: u64,
    sheet_names: Vec<String>,
}

#[derive(Template)]
#[template(path = "sample.html")]
struct SampleTemplate {
    sheets: Vec<SampleSheetData>,
}

#[derive(Serialize)]
struct SampleSheetData {
    sheet_name: String,
    headers: Vec<String>,
    rows: Vec<care_shelter_donation_aggregation::SampleRow>,
}

#[derive(Template)]
#[template(path = "about.html")]
struct AboutTemplate {
    git_hash: String,
    git_branch: String,
    git_date: String,
}

#[derive(Template)]
#[template(path = "success.html")]
struct SuccessTemplate {
    session_id: String,
    total_rows: usize,
    sheets_processed: usize,
    warnings: Vec<String>,
    deduplication_log: String,
    records_before_dedup: usize,
    duplicates_removed: usize,
}

#[derive(Serialize, ToSchema)]
struct MappingDisplay {
    sheet_name: String,
    mappings: Vec<MappingItem>,
}

#[derive(Serialize, ToSchema)]
struct MappingItem {
    source: String,
    target: String,
    description: String,
}

#[derive(Serialize, ToSchema)]
struct OrphanMappingDisplay {
    sheet_name: String,
    unmapped_fields: Vec<OrphanFieldItem>,
}

#[derive(Serialize, ToSchema)]
struct OrphanFieldItem {
    field_name: String,
    description: String,
}

#[derive(Template)]
#[template(path = "orphan_mappings.html")]
struct OrphanMappingsTemplate {}

#[derive(OpenApi)]
#[openapi(
    paths(
        mappings_api,
        orphan_mappings_api,
    ),
    components(
        schemas(MappingDisplay, MappingItem, OrphanMappingDisplay, OrphanFieldItem)
    ),
    tags(
        (name = "Mappings", description = "Field mapping information endpoints")
    ),
    info(
        title = "C.A.R.E. Shelter Donation Data Aggregation API",
        version = "1.0.0",
        description = "API for processing donation data from multiple sources into standardized DonorSnap format.\n\n**Privacy Notice:** This service does not store any uploaded data. All processing is done in memory.",
        contact(
            name = "Henry Post",
            url = "http://henrypost.github.io/"
        )
    )
)]
struct ApiDoc;

struct ProcessResult {
    csv_data: String,
    warnings: Vec<String>,
    total_rows: usize,
    sheets_processed: usize,
    deduplication_log: String,
    records_before_dedup: usize,
    duplicates_removed: usize,
}

// PRIVACY: Download link expiration time in seconds
const DOWNLOAD_EXPIRY_SECONDS: u64 = 60; // 1 minute

// Temporary storage for CSV files with automatic cleanup
// PRIVACY: Data is stored only in memory and automatically cleaned up after 1 minute or on download
#[derive(Clone)]
struct CsvStorage {
    data: Arc<Mutex<HashMap<String, (String, Instant)>>>,
}

impl CsvStorage {
    fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn store(&self, csv_data: String) -> String {
        let session_id = Uuid::new_v4().to_string();
        let mut storage = self.data.lock().unwrap();

        // Clean up old entries (older than DOWNLOAD_EXPIRY_SECONDS)
        let now = Instant::now();
        storage.retain(|_, (_, timestamp)| {
            now.duration_since(*timestamp) < Duration::from_secs(DOWNLOAD_EXPIRY_SECONDS)
        });

        storage.insert(session_id.clone(), (csv_data, now));
        session_id
    }

    fn retrieve_and_remove(&self, session_id: &str) -> Option<String> {
        let mut storage = self.data.lock().unwrap();
        storage.remove(session_id).map(|(csv_data, _)| csv_data)
    }
}

#[tokio::main]
async fn main() {
    let csv_storage = CsvStorage::new();

    let app = Router::new()
        .route("/", get(home_page))
        .route("/about", get(about_page))
        .route("/mappings", get(mappings_page))
        .route("/api/mappings", get(mappings_api))
        .route("/orphan-mappings", get(orphan_mappings_page))
        .route("/api/orphan-mappings", get(orphan_mappings_api))
        .merge(SwaggerUi::new("/openapi").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/faq", get(faq_page))
        .route("/sample", get(sample_page))
        .route("/sample/download/:sheet_name", get(download_sample_csv))
        .route("/upload", post(upload_file))
        .route("/download/:session_id", get(download_csv))
        .with_state(csv_storage);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Server running at http://localhost:8080");
    axum::serve(listener, app).await.unwrap();
}

async fn home_page() -> HomeTemplate {
    let sheet_names: Vec<String> = get_all_sheet_mappings()
        .iter()
        .map(|m| m.sheet_name.to_string())
        .collect();

    HomeTemplate { sheet_names }
}

fn get_display_mappings() -> Vec<MappingDisplay> {
    let mappings = get_all_sheet_mappings();
    let field_descriptions = get_field_descriptions();

    let mut field_desc_map: HashMap<&str, &str> = HashMap::new();
    for desc in &field_descriptions {
        field_desc_map.insert(desc.target_field, desc.description);
    }

    mappings
        .iter()
        .map(|sheet_mapping| {
            let items: Vec<MappingItem> = sheet_mapping
                .mappings
                .iter()
                .map(|(target, source)| MappingItem {
                    source: source.to_string(),
                    target: target.to_string(),
                    description: field_desc_map
                        .get(target)
                        .unwrap_or(&"")
                        .to_string(),
                })
                .collect();

            MappingDisplay {
                sheet_name: sheet_mapping.sheet_name.to_string(),
                mappings: items,
            }
        })
        .collect()
}

async fn mappings_page() -> MappingsTemplate {
    MappingsTemplate {}
}

#[utoipa::path(
    get,
    path = "/api/mappings",
    tag = "Mappings",
    responses(
        (status = 200, description = "List all field mappings", body = Vec<MappingDisplay>)
    )
)]
async fn mappings_api() -> Json<Vec<MappingDisplay>> {
    Json(get_display_mappings())
}

fn get_orphan_mappings() -> Vec<OrphanMappingDisplay> {
    let mappings = get_all_sheet_mappings();
    let field_descriptions = get_field_descriptions();

    let mut field_desc_map: HashMap<&str, &str> = HashMap::new();
    for desc in &field_descriptions {
        field_desc_map.insert(desc.target_field, desc.description);
    }

    mappings
        .iter()
        .map(|sheet_mapping| {
            // Get the set of mapped target fields for this sheet
            let mapped_targets: std::collections::HashSet<&str> = sheet_mapping
                .mappings
                .iter()
                .map(|(target, _)| *target)
                .collect();

            // Find unmapped fields (fields in DONORSNAP_FIELDS_WE_CARE_ABOUT but not in mapped_targets)
            let unmapped_fields: Vec<OrphanFieldItem> = DONORSNAP_FIELDS_WE_CARE_ABOUT
                .iter()
                .filter(|field| !mapped_targets.contains(*field))
                .map(|field| OrphanFieldItem {
                    field_name: field.to_string(),
                    description: field_desc_map.get(field).unwrap_or(&"").to_string(),
                })
                .collect();

            OrphanMappingDisplay {
                sheet_name: sheet_mapping.sheet_name.to_string(),
                unmapped_fields,
            }
        })
        .collect()
}

async fn orphan_mappings_page() -> OrphanMappingsTemplate {
    OrphanMappingsTemplate {}
}

#[utoipa::path(
    get,
    path = "/api/orphan-mappings",
    tag = "Mappings",
    responses(
        (status = 200, description = "List orphan mappings (unmapped fields)", body = Vec<OrphanMappingDisplay>)
    )
)]
async fn orphan_mappings_api() -> Json<Vec<OrphanMappingDisplay>> {
    Json(get_orphan_mappings())
}

async fn about_page() -> AboutTemplate {
    AboutTemplate {
        git_hash: env!("GIT_HASH").to_string(),
        git_branch: env!("GIT_BRANCH").to_string(),
        git_date: env!("GIT_DATE").to_string(),
    }
}

async fn faq_page() -> FaqTemplate {
    let sheet_names: Vec<String> = get_all_sheet_mappings()
        .iter()
        .map(|m| m.sheet_name.to_string())
        .collect();

    FaqTemplate {
        download_expiry_seconds: DOWNLOAD_EXPIRY_SECONDS,
        sheet_names,
    }
}

async fn sample_page() -> SampleTemplate {
    let mappings = get_all_sheet_mappings();
    let mut sheets = Vec::new();

    for mapping in mappings {
        let (headers, rows) = care_shelter_donation_aggregation::generate_sample_data_for_sheet(&mapping, 2);
        sheets.push(SampleSheetData {
            sheet_name: mapping.sheet_name.to_string(),
            headers,
            rows,
        });
    }

    SampleTemplate { sheets }
}

async fn download_sample_csv(Path(sheet_name): Path<String>) -> Response {
    // Find the matching sheet mapping
    let mappings = get_all_sheet_mappings();
    let sheet_mapping = mappings.iter().find(|m| m.sheet_name == sheet_name);

    match sheet_mapping {
        Some(mapping) => {
            // Generate 10 rows of sample data
            let (headers, rows) = care_shelter_donation_aggregation::generate_sample_data_for_sheet(mapping, 10);

            // Create CSV
            let mut csv_output = Vec::new();
            {
                let mut wtr = Writer::from_writer(&mut csv_output);

                // Write headers
                if let Err(e) = wtr.write_record(&headers) {
                    let error_template = ErrorTemplate {
                        error_message: format!("Failed to generate sample CSV: {}", e),
                    };
                    return error_template.into_response();
                }

                // Write data rows
                for row in rows {
                    if let Err(e) = wtr.write_record(&row.cells) {
                        let error_template = ErrorTemplate {
                            error_message: format!("Failed to generate sample CSV: {}", e),
                        };
                        return error_template.into_response();
                    }
                }

                if let Err(e) = wtr.flush() {
                    let error_template = ErrorTemplate {
                        error_message: format!("Failed to generate sample CSV: {}", e),
                    };
                    return error_template.into_response();
                }
            }

            let csv_data = match String::from_utf8(csv_output) {
                Ok(data) => data,
                Err(e) => {
                    let error_template = ErrorTemplate {
                        error_message: format!("Failed to encode sample CSV: {}", e),
                    };
                    return error_template.into_response();
                }
            };

            // Return CSV with appropriate headers
            (
                [
                    ("Content-Type", "text/csv"),
                    ("Content-Disposition", &format!("attachment; filename=\"sample_{}.csv\"", sheet_name.replace(" ", "_"))),
                ],
                csv_data,
            )
                .into_response()
        }
        None => {
            let error_template = ErrorTemplate {
                error_message: format!("Unknown sheet name: {}", sheet_name),
            };
            error_template.into_response()
        }
    }
}

async fn download_csv(
    State(storage): State<CsvStorage>,
    Path(session_id): Path<String>,
) -> Response {
    match storage.retrieve_and_remove(&session_id) {
        Some(csv_data) => {
            (
                [
                    ("Content-Type", "text/csv"),
                    ("Content-Disposition", "attachment; filename=\"aggregated_donations.csv\""),
                ],
                csv_data,
            )
                .into_response()
        }
        None => {
            let error_template = ErrorTemplate {
                error_message: format!(
                    "Download link has expired or is invalid.\n\n\
                    Download links expire after {} seconds ({} minute) or after the first download for security reasons.\n\n\
                    Please upload your file again to generate a new download.",
                    DOWNLOAD_EXPIRY_SECONDS,
                    DOWNLOAD_EXPIRY_SECONDS / 60
                ),
            };
            error_template.into_response()
        }
    }
}

async fn upload_file(
    State(storage): State<CsvStorage>,
    mut multipart: Multipart,
) -> Response {
    while let Some(field) = multipart.next_field().await.unwrap() {
        if field.name() == Some("file") {
            let filename = field.file_name().unwrap_or("unknown").to_string();
            let data = match field.bytes().await {
                Ok(d) => d,
                Err(e) => {
                    let error_template = ErrorTemplate {
                        error_message: format!(
                            "Failed to read uploaded file '{}': {}\n\nThe file may be corrupted or too large.",
                            filename, e
                        ),
                    };
                    return error_template.into_response();
                }
            };

            if data.is_empty() {
                let error_template = ErrorTemplate {
                    error_message: format!(
                        "The uploaded file '{}' is empty.\n\nPlease upload a valid Excel file containing donation data.",
                        filename
                    ),
                };
                return error_template.into_response();
            }

            // PRIVACY: Save to temporary file that is automatically deleted
            // NamedTempFile creates a file that is removed when the TempPath goes out of scope
            // This ensures no uploaded data persists on disk after processing
            let temp_file = match NamedTempFile::new() {
                Ok(f) => f,
                Err(e) => {
                    let error_template = ErrorTemplate {
                        error_message: format!(
                            "Server error: Failed to create temporary file for processing: {}\n\nPlease try again or contact support if the problem persists.",
                            e
                        ),
                    };
                    return error_template.into_response();
                }
            };
            let (mut file, path) = temp_file.into_parts();

            if let Err(e) = tokio::task::block_in_place(|| {
                std::io::Write::write_all(&mut file, &data)
            }) {
                let error_template = ErrorTemplate {
                    error_message: format!(
                        "Failed to save uploaded file for processing: {}\n\nThe file may be corrupted or your system may be out of disk space.",
                        e
                    ),
                };
                return error_template.into_response();
            }

            // Process the file
            match process_spreadsheet(path.to_str().unwrap(), &filename) {
                Ok(result) => {
                    // If there are warnings or deduplication log, show success page
                    if !result.warnings.is_empty() || !result.deduplication_log.is_empty() {
                        let session_id = storage.store(result.csv_data);
                        let success_template = SuccessTemplate {
                            session_id,
                            total_rows: result.total_rows,
                            sheets_processed: result.sheets_processed,
                            warnings: result.warnings,
                            deduplication_log: result.deduplication_log,
                            records_before_dedup: result.records_before_dedup,
                            duplicates_removed: result.duplicates_removed,
                        };
                        return success_template.into_response();
                    } else {
                        // No warnings or dedup log, return CSV directly for quick download
                        return (
                            [
                                ("Content-Type", "text/csv"),
                                ("Content-Disposition", "attachment; filename=\"aggregated_donations.csv\""),
                            ],
                            result.csv_data,
                        )
                            .into_response();
                    }
                }
                Err(e) => {
                    let error_template = ErrorTemplate {
                        error_message: e,
                    };
                    return error_template.into_response();
                }
            }
        }
    }

    let error_template = ErrorTemplate {
        error_message: "No file was uploaded.\n\nPlease select an Excel file (.xlsx or .xls) and try again.".to_string(),
    };
    error_template.into_response()
}

fn process_spreadsheet(file_path: &str, filename: &str) -> Result<ProcessResult, String> {
    // First, try to open the workbook to validate it's a proper Excel file
    if let Err(e) = open_workbook_auto(file_path) {
        return Err(format!(
            "Failed to open '{}' as an Excel file: {}\n\n\
            Possible causes:\n\
            • The file is not a valid Excel file (.xlsx or .xls)\n\
            • The file is corrupted\n\
            • The file format is not supported\n\n\
            Please ensure you're uploading a valid Excel workbook.",
            filename, e
        ));
    }

    let mut all_sheet_records: Vec<(String, Vec<StringRecord>)> = Vec::new();
    let mut processed_sheets = Vec::new();
    let mut failed_sheets = Vec::new();
    let mut warnings = Vec::new();

    // Step 1: Extract and normalize all records from all sheets
    let mappings = get_all_sheet_mappings();
    for sheet_mapping in mappings {
        match extract_sheet_data(file_path, &sheet_mapping) {
            Ok((records, sheet_warnings)) => {
                if !records.is_empty() {
                    processed_sheets.push((sheet_mapping.sheet_name.to_string(), records.len()));
                    all_sheet_records.push((sheet_mapping.sheet_name.to_string(), records));

                    // Collect warnings from this sheet
                    for warning in sheet_warnings {
                        warnings.push(warning);
                    }
                }
            }
            Err(e) => {
                failed_sheets.push((sheet_mapping.sheet_name.to_string(), e));
            }
        }
    }

    // If no sheets were successfully processed, return detailed error
    if processed_sheets.is_empty() {
        let mut error_msg = format!(
            "No data could be extracted from '{}'.\n\n",
            filename
        );

        if !failed_sheets.is_empty() {
            error_msg.push_str("The following sheets had problems:\n\n");
            for (sheet_name, error) in &failed_sheets {
                error_msg.push_str(&format!("• {}: {}\n", sheet_name, error));
            }
            error_msg.push_str("\n");
        }

        error_msg.push_str("Please ensure:\n\
            • Your file contains at least one sheet with a recognized name\n\
            • The sheet has the correct column headers (see the mappings page)\n\
            • The sheet contains data rows (not just headers)");

        return Err(error_msg);
    }

    // Add warnings for sheets that failed processing
    if !failed_sheets.is_empty() {
        for (sheet_name, error) in &failed_sheets {
            warnings.push(format!("Sheet '{}': {}", sheet_name, error));
        }
    }

    // Step 2: Apply deduplication across all sheets
    let headers = StringRecord::from(DONORSNAP_FIELDS_WE_CARE_ABOUT.to_vec());
    let dedup_result = deduplicate_multi_sheet(&headers, all_sheet_records);

    // Add any deduplication errors to warnings
    for error in &dedup_result.errors {
        warnings.push(format!("Deduplication error: {}", error));
    }

    // Step 3: Write deduplicated records to CSV
    let mut csv_output = Vec::new();
    {
        let mut wtr = Writer::from_writer(&mut csv_output);

        // Write header
        if let Err(e) = wtr.write_record(DONORSNAP_FIELDS_WE_CARE_ABOUT) {
            return Err(format!(
                "Internal error writing CSV header: {}\n\nPlease try again or contact support.",
                e
            ));
        }

        // Write deduplicated records
        for record in &dedup_result.records {
            if let Err(e) = wtr.write_record(record) {
                return Err(format!(
                    "Failed to write record to CSV: {}\n\nPlease try again or contact support.",
                    e
                ));
            }
        }

        if let Err(e) = wtr.flush() {
            return Err(format!(
                "Failed to finalize CSV output: {}\n\nPlease try again or contact support.",
                e
            ));
        }
    }

    let csv_data = String::from_utf8(csv_output).map_err(|e| {
        format!(
            "Failed to encode CSV output as UTF-8: {}\n\n\
            Your data may contain unsupported characters.",
            e
        )
    })?;

    Ok(ProcessResult {
        csv_data,
        warnings,
        total_rows: dedup_result.records.len(),
        sheets_processed: processed_sheets.len(),
        deduplication_log: dedup_result.log,
        records_before_dedup: dedup_result.records_before_dedup,
        duplicates_removed: dedup_result.duplicates_removed,
    })
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

fn extract_sheet_data(
    file_path: &str,
    sheet_mapping: &care_shelter_donation_aggregation::SheetMapping,
) -> Result<(Vec<StringRecord>, Vec<String>), String> {
    let mut workbook = open_workbook_auto(file_path).map_err(|e| {
        format!("Failed to open workbook: {}", e)
    })?;

    let mut warnings = Vec::new();

    // Try to get the worksheet
    let sheet = match workbook.worksheet_range(sheet_mapping.sheet_name) {
        Ok(s) => s,
        Err(_) => {
            // Sheet doesn't exist - this is not necessarily an error if the file doesn't contain this source
            return Ok((Vec::new(), warnings)); // Return empty records, no warnings
        }
    };

    let mut rows = sheet.rows();

    // Get header row to map column indices
    let headers = match rows.next() {
        Some(row) => row.iter().map(|cell| data_to_string(cell)).collect::<Vec<String>>(),
        None => {
            return Err(format!(
                "Sheet '{}' exists but has no header row.\n\
                Expected headers: {}",
                sheet_mapping.sheet_name,
                sheet_mapping.mappings.iter()
                    .map(|(_, src)| *src)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    };

    // Create mapping of target field names to column indices
    let field_mappings = sheet_mapping.to_hashmap();
    let mut field_indices = HashMap::new();
    let mut missing_columns = Vec::new();

    for (&target_field, &source_field) in &field_mappings {
        if let Some(index) = headers.iter().position(|h| h == source_field) {
            field_indices.insert(target_field, index);
        } else {
            // Track missing columns but don't fail - some columns may be optional
            missing_columns.push(source_field);
        }
    }

    // If ALL expected columns are missing, return an error
    if field_indices.is_empty() && !field_mappings.is_empty() {
        return Err(format!(
            "None of the expected columns were found in sheet '{}'.\n\n\
            Expected columns:\n{}\n\n\
            Found columns:\n{}\n\n\
            Please check the mappings page to see the correct column names for this sheet.",
            sheet_mapping.sheet_name,
            field_mappings.values()
                .map(|s| format!("  • {}", s))
                .collect::<Vec<_>>()
                .join("\n"),
            headers.iter()
                .map(|h| format!("  • {}", h))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    // Find indices for phone and state fields for normalization
    let phone_idx = DONORSNAP_FIELDS_WE_CARE_ABOUT.iter().position(|&f| f == "Phone");
    let state_idx = DONORSNAP_FIELDS_WE_CARE_ABOUT.iter().position(|&f| f == "State/Province");

    // Process data rows
    let mut records = Vec::new();
    for row in rows {
        let mut record_data = Vec::new();
        let mut has_any_data = false;

        for (field_idx, field) in DONORSNAP_FIELDS_WE_CARE_ABOUT.iter().enumerate() {
            let mut value = match field_indices.get(field) {
                Some(&index) => {
                    if index < row.len() {
                        let val = data_to_string(&row[index]);
                        if !val.is_empty() {
                            has_any_data = true;
                        }
                        val
                    } else {
                        String::new()
                    }
                }
                None => String::new(),
            };

            // Apply normalization
            if Some(field_idx) == phone_idx && !value.is_empty() {
                value = normalize_phone(&value);
            } else if Some(field_idx) == state_idx && !value.is_empty() {
                value = normalize_state(&value);
            }

            record_data.push(value);
        }

        // Only include rows that have at least some data
        if has_any_data {
            records.push(StringRecord::from(record_data));
        }
    }

    // Warn about missing columns if some data was found
    if !records.is_empty() && !missing_columns.is_empty() {
        warnings.push(format!(
            "Sheet '{}' is missing some optional columns: {}",
            sheet_mapping.sheet_name,
            missing_columns.join(", ")
        ));
    }

    Ok((records, warnings))
}
