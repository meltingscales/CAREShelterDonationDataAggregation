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

use askama::Template;
use axum::{
    extract::{Multipart, Path, State},
    response::{IntoResponse, Response, Json},
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;
use tower_http::limit::RequestBodyLimitLayer;
use tower::ServiceBuilder;
use calamine::{open_workbook_auto, Data, Reader};
use care_shelter_donation_aggregation::{
    get_all_sheet_mappings, get_field_descriptions, DONORSNAP_FIELDS_WE_CARE_ABOUT,
    normalize_phone, normalize_state, deduplicate_multi_sheet, FieldDescription,
    DEDUPLICATION_PRIORITY, get_algorithms, apply_all_algorithms, NameSplitAlgorithm,
    read_all_sheets, write_xlsx_to_bytes, deduplicate_sheet_rows, data_to_string,
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
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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

#[derive(Serialize, ToSchema)]
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
    has_duplicates: bool,
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

// API-specific structures
#[derive(Serialize, ToSchema)]
struct ApiProcessResult {
    csv_data: String,
    warnings: Vec<String>,
    total_rows: usize,
    sheets_processed: usize,
    deduplication_log: String,
    records_before_dedup: usize,
    duplicates_removed: usize,
}

#[derive(Serialize, ToSchema)]
struct ApiValidationResult {
    valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
    found_sheets: Vec<String>,
    missing_sheets: Vec<String>,
}

#[derive(Serialize, ToSchema)]
struct VersionInfo {
    git_hash: String,
    git_branch: String,
    git_date: String,
}

#[derive(Serialize, ToSchema)]
struct HealthResponse {
    status: String,
}

#[derive(Serialize, ToSchema)]
struct SheetsResponse {
    sheets: Vec<String>,
}

#[derive(Template)]
#[template(path = "orphan_mappings.html")]
struct OrphanMappingsTemplate {}

#[derive(Template)]
#[template(path = "name_splitter.html")]
struct NameSplitterTemplate {
    algorithms: Vec<NameSplitAlgorithm>,
}

#[derive(Serialize, ToSchema)]
struct NameSplitterResult {
    csv_data: String,
    total_rows: usize,
    name_column: String,
}

#[derive(Template)]
#[template(path = "email_name_dedup.html")]
struct EmailNameDedupTemplate {}

#[derive(Template)]
#[template(path = "email_name_dedup_success.html")]
struct EmailNameDedupSuccessTemplate {
    session_id: String,
    total_sheets: usize,
    total_rows_before: usize,
    total_rows_after: usize,
    total_duplicates_removed: usize,
    deduplication_log: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        mappings_api,
        orphan_mappings_api,
        upload_api,
        validate_api,
        version_api,
        health_api,
        fields_api,
        deduplication_priority_api,
        sheets_api,
        sample_api,
        name_splitter_algorithms_api,
        name_splitter_process_api,
    ),
    components(
        schemas(
            MappingDisplay,
            MappingItem,
            OrphanMappingDisplay,
            OrphanFieldItem,
            ApiProcessResult,
            ApiValidationResult,
            VersionInfo,
            HealthResponse,
            FieldDescription,
            DeduplicationPriorityResponse,
            SheetsResponse,
            SampleSheetData,
            NameSplitAlgorithm,
            NameSplitterResult,
        )
    ),
    tags(
        (name = "Mappings", description = "Field mapping information endpoints"),
        (name = "Processing", description = "File upload and processing endpoints"),
        (name = "Validation", description = "File validation endpoints"),
        (name = "System", description = "System health and version information"),
        (name = "Reference", description = "Reference data and sample information"),
        (name = "Utilities", description = "Utility tools for data processing"),
    ),
    info(
        title = "C.A.R.E. Shelter Donation Data Aggregation API",
        version = "1.0.0",
        description = "API for processing donation data from multiple sources into standardized DonorSnap format.\n\n**Privacy Notice:** This service does not store any uploaded data. All processing is done in memory.",
        contact(
            name = "Henry Post",
            url = "http://henrypost.github.io/"
        ),
        license(
            name = "GPL-3.0",
            url = "https://www.gnu.org/licenses/gpl-3.0.html"
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
    removed_duplicates_csv: Option<String>,
}

// PRIVACY: Download link expiration time in seconds
const DOWNLOAD_EXPIRY_SECONDS: u64 = 60; // 1 minute

// Temporary storage for CSV and XLSX files with automatic cleanup
// PRIVACY: Data is stored only in memory and automatically cleaned up after 1 minute or on download
#[derive(Clone)]
struct CsvStorage {
    data: Arc<Mutex<HashMap<String, (String, Option<String>, String, Instant)>>>,
    xlsx_data: Arc<Mutex<HashMap<String, (Vec<u8>, String, Instant)>>>,
}

impl CsvStorage {
    fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
            xlsx_data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn store(&self, csv_data: String, duplicates_csv: Option<String>, log: String) -> String {
        let session_id = Uuid::new_v4().to_string();
        let mut storage = self.data.lock().unwrap();

        // Clean up old entries (older than DOWNLOAD_EXPIRY_SECONDS)
        let now = Instant::now();
        storage.retain(|_, (_, _, _, timestamp)| {
            now.duration_since(*timestamp) < Duration::from_secs(DOWNLOAD_EXPIRY_SECONDS)
        });

        storage.insert(session_id.clone(), (csv_data, duplicates_csv, log, now));
        session_id
    }

    fn retrieve_and_remove(&self, session_id: &str) -> Option<String> {
        let storage = self.data.lock().unwrap();
        storage.get(session_id).map(|(csv_data, _, _, _)| csv_data.clone())
    }

    fn retrieve_duplicates_and_remove(&self, session_id: &str) -> Option<String> {
        let storage = self.data.lock().unwrap();
        storage.get(session_id).and_then(|(_, duplicates_csv, _, _)| duplicates_csv.clone())
    }

    fn retrieve_log_and_remove(&self, session_id: &str) -> Option<String> {
        let storage = self.data.lock().unwrap();
        storage.get(session_id).map(|(_, _, log, _)| log.clone())
    }

    fn remove_session(&self, session_id: &str) {
        let mut storage = self.data.lock().unwrap();
        storage.remove(session_id);
    }

    fn store_xlsx(&self, xlsx_data: Vec<u8>, log: String) -> String {
        let session_id = Uuid::new_v4().to_string();
        let mut storage = self.xlsx_data.lock().unwrap();

        // Clean up old entries (older than DOWNLOAD_EXPIRY_SECONDS)
        let now = Instant::now();
        storage.retain(|_, (_, _, timestamp)| {
            now.duration_since(*timestamp) < Duration::from_secs(DOWNLOAD_EXPIRY_SECONDS)
        });

        storage.insert(session_id.clone(), (xlsx_data, log, now));
        session_id
    }

    fn retrieve_xlsx(&self, session_id: &str) -> Option<Vec<u8>> {
        let storage = self.xlsx_data.lock().unwrap();
        storage.get(session_id).map(|(xlsx_data, _, _)| xlsx_data.clone())
    }

    fn retrieve_xlsx_log(&self, session_id: &str) -> Option<String> {
        let storage = self.xlsx_data.lock().unwrap();
        storage.get(session_id).map(|(_, log, _)| log.clone())
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging (disable in Cloud Run to avoid startup issues)
    // Cloud Run sets K_SERVICE environment variable
    if std::env::var("K_SERVICE").is_err() {
        tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new("info"))
            )
            .with(tracing_subscriber::fmt::layer().compact())
            .init();
    }

    tracing::info!("Starting C.A.R.E. Shelter Donation Data Aggregation server");

    let csv_storage = CsvStorage::new();

    let app = Router::new()
        .route("/", get(home_page))
        .route("/about", get(about_page))
        .route("/mappings", get(mappings_page))
        .route("/api/mappings", get(mappings_api))
        .route("/orphan-mappings", get(orphan_mappings_page))
        .route("/api/orphan-mappings", get(orphan_mappings_api))
        .route("/api/upload", post(upload_api))
        .route("/api/validate", post(validate_api))
        .route("/api/version", get(version_api))
        .route("/api/health", get(health_api))
        .route("/api/fields", get(fields_api))
        .route("/api/deduplication-priority", get(deduplication_priority_api))
        .route("/api/sheets", get(sheets_api))
        .route("/api/sample/:sheet_name", get(sample_api))
        .route("/api/name-splitter/algorithms", get(name_splitter_algorithms_api))
        .route("/api/name-splitter/process", post(name_splitter_process_api))
        .merge(SwaggerUi::new("/openapi").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/faq", get(faq_page))
        .route("/sample", get(sample_page))
        .route("/sample/download/:sheet_name", get(download_sample_csv))
        .route("/name-splitter", get(name_splitter_page))
        .route("/name-splitter/process", post(process_name_splitter))
        .route("/email-name-dedup", get(email_name_dedup_page))
        .route("/email-name-dedup/process", post(process_email_name_dedup))
        .route("/download-xlsx/:session_id", get(download_xlsx))
        .route("/download-xlsx-log/:session_id", get(download_xlsx_log))
        .route("/upload", post(upload_file))
        .route("/download/:session_id", get(download_csv))
        .route("/download-duplicates/:session_id", get(download_duplicates_csv))
        .route("/download-log/:session_id", get(download_log))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(csv_storage)
        .layer(
            ServiceBuilder::new()
                // Request body size limit: 100MB
                .layer(RequestBodyLimitLayer::new(100 * 1024 * 1024))
        );

    // Cloud Run provides PORT environment variable, default to 8080
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Server listening on {}", addr);

    // Graceful shutdown handler
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, shutting down gracefully");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, shutting down gracefully");
        },
    }
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

// API Endpoints

#[utoipa::path(
    post,
    path = "/api/upload",
    tag = "Processing",
    responses(
        (status = 200, description = "File processed successfully", body = ApiProcessResult),
        (status = 400, description = "Invalid file or processing error")
    )
)]
async fn upload_api(mut multipart: Multipart) -> Response {
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("Failed to parse multipart data: {}", e)
                    }))
                ).into_response();
            }
        };

        if field.name() == Some("file") {
            let filename = field.file_name().unwrap_or("unknown").to_string();
            let data = match field.bytes().await {
                Ok(d) => d,
                Err(e) => {
                    return (
                        axum::http::StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": format!("Failed to read uploaded file: {}", e)
                        }))
                    ).into_response();
                }
            };

            if data.is_empty() {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Uploaded file is empty"
                    }))
                ).into_response();
            }

            // Save to temporary file
            let temp_file = match NamedTempFile::new() {
                Ok(f) => f,
                Err(e) => {
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": format!("Failed to create temporary file: {}", e)
                        }))
                    ).into_response();
                }
            };
            let (mut file, path) = temp_file.into_parts();

            if let Err(e) = tokio::task::block_in_place(|| {
                std::io::Write::write_all(&mut file, &data)
            }) {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": format!("Failed to save file: {}", e)
                    }))
                ).into_response();
            }

            // Process the file
            match process_spreadsheet(path.to_str().unwrap(), &filename) {
                Ok(result) => {
                    let api_result = ApiProcessResult {
                        csv_data: result.csv_data,
                        warnings: result.warnings,
                        total_rows: result.total_rows,
                        sheets_processed: result.sheets_processed,
                        deduplication_log: result.deduplication_log,
                        records_before_dedup: result.records_before_dedup,
                        duplicates_removed: result.duplicates_removed,
                    };
                    return (axum::http::StatusCode::OK, Json(api_result)).into_response();
                }
                Err(e) => {
                    return (
                        axum::http::StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": e
                        }))
                    ).into_response();
                }
            }
        }
    }

    (
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": "No file uploaded"
        }))
    ).into_response()
}

#[utoipa::path(
    post,
    path = "/api/validate",
    tag = "Validation",
    responses(
        (status = 200, description = "Validation result", body = ApiValidationResult),
        (status = 400, description = "Invalid file")
    )
)]
async fn validate_api(mut multipart: Multipart) -> Response {
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("Failed to parse multipart data: {}", e)
                    }))
                ).into_response();
            }
        };

        if field.name() == Some("file") {
            let filename = field.file_name().unwrap_or("unknown").to_string();
            let data = match field.bytes().await {
                Ok(d) => d,
                Err(e) => {
                    return (
                        axum::http::StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": format!("Failed to read uploaded file: {}", e)
                        }))
                    ).into_response();
                }
            };

            // Save to temporary file
            let temp_file = match NamedTempFile::new() {
                Ok(f) => f,
                Err(e) => {
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": format!("Failed to create temporary file: {}", e)
                        }))
                    ).into_response();
                }
            };
            let (mut file, path) = temp_file.into_parts();

            if let Err(e) = tokio::task::block_in_place(|| {
                std::io::Write::write_all(&mut file, &data)
            }) {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": format!("Failed to save file: {}", e)
                    }))
                ).into_response();
            }

            // Validate the file
            let validation_result = validate_spreadsheet(path.to_str().unwrap(), &filename);
            return (axum::http::StatusCode::OK, Json(validation_result)).into_response();
        }
    }

    (
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": "No file uploaded"
        }))
    ).into_response()
}

#[utoipa::path(
    get,
    path = "/api/version",
    tag = "System",
    responses(
        (status = 200, description = "Version information", body = VersionInfo)
    )
)]
async fn version_api() -> Json<VersionInfo> {
    Json(VersionInfo {
        git_hash: env!("GIT_HASH").to_string(),
        git_branch: env!("GIT_BRANCH").to_string(),
        git_date: env!("GIT_DATE").to_string(),
    })
}

#[utoipa::path(
    get,
    path = "/api/health",
    tag = "System",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
async fn health_api() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

#[utoipa::path(
    get,
    path = "/api/fields",
    tag = "Reference",
    responses(
        (status = 200, description = "List of all DonorSnap target fields", body = Vec<FieldDescription>)
    )
)]
async fn fields_api() -> Json<Vec<FieldDescription>> {
    Json(get_field_descriptions())
}

#[derive(Serialize, ToSchema)]
struct DeduplicationPriorityResponse {
    /// Ordered list of sheet names by priority (index 0 = highest priority)
    priority_order: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/api/deduplication-priority",
    tag = "Reference",
    responses(
        (status = 200, description = "Deduplication priority order (lower index = higher priority)", body = DeduplicationPriorityResponse)
    )
)]
async fn deduplication_priority_api() -> Json<DeduplicationPriorityResponse> {
    let priority_order: Vec<String> = DEDUPLICATION_PRIORITY
        .iter()
        .map(|s| s.to_string())
        .collect();

    Json(DeduplicationPriorityResponse { priority_order })
}

#[utoipa::path(
    get,
    path = "/api/sheets",
    tag = "Reference",
    responses(
        (status = 200, description = "List of supported sheet names", body = SheetsResponse)
    )
)]
async fn sheets_api() -> Json<SheetsResponse> {
    let sheets: Vec<String> = get_all_sheet_mappings()
        .iter()
        .map(|m| m.sheet_name.to_string())
        .collect();

    Json(SheetsResponse { sheets })
}

#[utoipa::path(
    get,
    path = "/api/sample/{sheet_name}",
    tag = "Reference",
    params(
        ("sheet_name" = String, Path, description = "Name of the sheet to get sample data for")
    ),
    responses(
        (status = 200, description = "Sample data for the specified sheet", body = SampleSheetData),
        (status = 404, description = "Sheet not found")
    )
)]
async fn sample_api(Path(sheet_name): Path<String>) -> Response {
    let mappings = get_all_sheet_mappings();
    let sheet_mapping = mappings.iter().find(|m| m.sheet_name == sheet_name);

    match sheet_mapping {
        Some(mapping) => {
            let (headers, rows) = care_shelter_donation_aggregation::generate_sample_data_for_sheet(mapping, 5);
            let sample_data = SampleSheetData {
                sheet_name: mapping.sheet_name.to_string(),
                headers,
                rows,
            };
            (axum::http::StatusCode::OK, Json(sample_data)).into_response()
        }
        None => {
            (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Sheet '{}' not found", sheet_name)
                }))
            ).into_response()
        }
    }
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

async fn download_duplicates_csv(
    State(storage): State<CsvStorage>,
    Path(session_id): Path<String>,
) -> Response {
    match storage.retrieve_duplicates_and_remove(&session_id) {
        Some(csv_data) => {
            (
                [
                    ("Content-Type", "text/csv"),
                    ("Content-Disposition", "attachment; filename=\"removed_duplicates.csv\""),
                ],
                csv_data,
            )
                .into_response()
        }
        None => {
            let error_template = ErrorTemplate {
                error_message: format!(
                    "Download link has expired, is invalid, or no duplicates were found.\n\n\
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

async fn download_log(
    State(storage): State<CsvStorage>,
    Path(session_id): Path<String>,
) -> Response {
    match storage.retrieve_log_and_remove(&session_id) {
        Some(log_data) => {
            (
                [
                    ("Content-Type", "text/plain; charset=utf-8"),
                    ("Content-Disposition", "attachment; filename=\"deduplication_log.txt\""),
                ],
                log_data,
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
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => {
                let error_template = ErrorTemplate {
                    error_message: format!(
                        "Failed to parse uploaded file: {}\n\nThe request may be malformed or corrupted.",
                        e
                    ),
                };
                return error_template.into_response();
            }
        };

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
                        let session_id = storage.store(result.csv_data, result.removed_duplicates_csv, result.deduplication_log.clone());
                        let has_duplicates = result.duplicates_removed > 0;
                        let success_template = SuccessTemplate {
                            session_id,
                            total_rows: result.total_rows,
                            sheets_processed: result.sheets_processed,
                            warnings: result.warnings,
                            deduplication_log: result.deduplication_log,
                            records_before_dedup: result.records_before_dedup,
                            duplicates_removed: result.duplicates_removed,
                            has_duplicates,
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

fn validate_spreadsheet(file_path: &str, filename: &str) -> ApiValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut found_sheets = Vec::new();
    let mut missing_sheets = Vec::new();

    // Try to open the workbook
    let mut workbook = match open_workbook_auto(file_path) {
        Ok(wb) => wb,
        Err(e) => {
            errors.push(format!("Failed to open '{}' as an Excel file: {}", filename, e));
            return ApiValidationResult {
                valid: false,
                errors,
                warnings,
                found_sheets,
                missing_sheets,
            };
        }
    };

    // Check which sheets exist
    let mappings = get_all_sheet_mappings();
    for sheet_mapping in &mappings {
        match workbook.worksheet_range(sheet_mapping.sheet_name) {
            Ok(sheet) => {
                found_sheets.push(sheet_mapping.sheet_name.to_string());

                // Check if sheet has headers
                let mut rows = sheet.rows();
                if let Some(header_row) = rows.next() {
                    let headers: Vec<String> = header_row.iter().map(|cell| data_to_string(cell)).collect();

                    // Check for expected columns
                    let field_mappings = sheet_mapping.to_hashmap();
                    let mut missing_columns = Vec::new();

                    for (_target, source) in &field_mappings {
                        if !headers.iter().any(|h| h == source) {
                            missing_columns.push(*source);
                        }
                    }

                    if !missing_columns.is_empty() {
                        warnings.push(format!(
                            "Sheet '{}' is missing columns: {}",
                            sheet_mapping.sheet_name,
                            missing_columns.join(", ")
                        ));
                    }

                    // Check if sheet has data rows
                    if rows.next().is_none() {
                        warnings.push(format!("Sheet '{}' has no data rows", sheet_mapping.sheet_name));
                    }
                } else {
                    warnings.push(format!("Sheet '{}' has no header row", sheet_mapping.sheet_name));
                }
            }
            Err(_) => {
                missing_sheets.push(sheet_mapping.sheet_name.to_string());
            }
        }
    }

    // Validation passes if we found at least one sheet
    let valid = !found_sheets.is_empty() && errors.is_empty();

    ApiValidationResult {
        valid,
        errors,
        warnings,
        found_sheets,
        missing_sheets,
    }
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

        // Write header with SourceSheet column
        let mut headers: Vec<&str> = DONORSNAP_FIELDS_WE_CARE_ABOUT.to_vec();
        headers.push("SourceSheet");
        if let Err(e) = wtr.write_record(&headers) {
            return Err(format!(
                "Internal error writing CSV header: {}\n\nPlease try again or contact support.",
                e
            ));
        }

        // Write deduplicated records with their source sheet
        for (record, sheet_name) in &dedup_result.records {
            let mut row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            row.push(sheet_name.clone());
            if let Err(e) = wtr.write_record(&row) {
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

    // Step 4: Generate CSV for removed duplicates (if any)
    let removed_duplicates_csv = if !dedup_result.removed_duplicates.is_empty() {
        let mut dup_csv_output = Vec::new();
        {
            let mut wtr = Writer::from_writer(&mut dup_csv_output);

            // Write extended header with "Source Sheet" column
            let mut dup_headers: Vec<&str> = DONORSNAP_FIELDS_WE_CARE_ABOUT.to_vec();
            dup_headers.push("SourceSheet");
            wtr.write_record(&dup_headers).map_err(|e| {
                format!("Failed to write duplicates CSV header: {}", e)
            })?;

            // Write removed duplicate records with their source sheet
            for (record, sheet_name) in &dedup_result.removed_duplicates {
                let mut row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
                row.push(sheet_name.clone());
                wtr.write_record(&row).map_err(|e| {
                    format!("Failed to write duplicate record to CSV: {}", e)
                })?;
            }

            wtr.flush().map_err(|e| {
                format!("Failed to finalize duplicates CSV: {}", e)
            })?;
        }

        let dup_csv_data = String::from_utf8(dup_csv_output).map_err(|e| {
            format!("Failed to encode duplicates CSV as UTF-8: {}", e)
        })?;

        Some(dup_csv_data)
    } else {
        None
    };

    Ok(ProcessResult {
        csv_data,
        warnings,
        total_rows: dedup_result.records.len(),
        sheets_processed: processed_sheets.len(),
        deduplication_log: dedup_result.log,
        records_before_dedup: dedup_result.records_before_dedup,
        duplicates_removed: dedup_result.duplicates_removed,
        removed_duplicates_csv,
    })
}

// data_to_string is now imported from xlsx_utils module

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

#[utoipa::path(
    get,
    path = "/api/name-splitter/algorithms",
    tag = "Utilities",
    responses(
        (status = 200, description = "List of available name splitting algorithms", body = Vec<NameSplitAlgorithm>)
    )
)]
async fn name_splitter_algorithms_api() -> Json<Vec<NameSplitAlgorithm>> {
    Json(get_algorithms())
}

#[utoipa::path(
    post,
    path = "/api/name-splitter/process",
    tag = "Utilities",
    responses(
        (status = 200, description = "Successfully processed spreadsheet with name splitting", body = NameSplitterResult),
        (status = 400, description = "Invalid file or processing error")
    )
)]
async fn name_splitter_process_api(mut multipart: Multipart) -> Response {
    let mut file_data: Option<bytes::Bytes> = None;
    let mut filename = String::from("unknown");
    let mut name_column: Option<String> = None;

    // Parse multipart form data
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("Failed to parse form data: {}", e)
                    }))
                ).into_response();
            }
        };

        match field.name() {
            Some("file") => {
                filename = field.file_name().unwrap_or("unknown").to_string();
                file_data = match field.bytes().await {
                    Ok(d) => Some(d),
                    Err(e) => {
                        return (
                            axum::http::StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({
                                "error": format!("Failed to read uploaded file: {}", e)
                            }))
                        ).into_response();
                    }
                };
            }
            Some("name_column") => {
                name_column = match field.text().await {
                    Ok(t) => Some(t),
                    Err(e) => {
                        return (
                            axum::http::StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({
                                "error": format!("Failed to read name column field: {}", e)
                            }))
                        ).into_response();
                    }
                };
            }
            _ => {}
        }
    }

    // Validate inputs
    let file_data = match file_data {
        Some(d) => d,
        None => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No file was uploaded"
                }))
            ).into_response();
        }
    };

    let name_column = match name_column {
        Some(c) if !c.is_empty() => c,
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Please specify the name of the column containing names to split"
                }))
            ).into_response();
        }
    };

    // Save to temporary file
    let temp_file = match NamedTempFile::new() {
        Ok(f) => f,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to create temporary file: {}", e)
                }))
            ).into_response();
        }
    };
    let (mut file, path) = temp_file.into_parts();

    if let Err(e) = tokio::task::block_in_place(|| {
        std::io::Write::write_all(&mut file, &file_data)
    }) {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to save file: {}", e)
            }))
        ).into_response();
    }

    // Process the file
    match process_name_splitting(path.to_str().unwrap(), &filename, &name_column) {
        Ok(csv_data) => {
            let total_rows = csv_data.lines().count().saturating_sub(1); // Subtract header
            let result = NameSplitterResult {
                csv_data,
                total_rows,
                name_column,
            };
            (axum::http::StatusCode::OK, Json(result)).into_response()
        }
        Err(e) => {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": e
                }))
            ).into_response()
        }
    }
}

async fn name_splitter_page() -> NameSplitterTemplate {
    NameSplitterTemplate {
        algorithms: get_algorithms(),
    }
}

async fn process_name_splitter(mut multipart: Multipart) -> Response {
    let mut file_data: Option<bytes::Bytes> = None;
    let mut filename = String::from("unknown");
    let mut name_column: Option<String> = None;

    // Parse multipart form data
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => {
                let error_template = ErrorTemplate {
                    error_message: format!("Failed to parse form data: {}", e),
                };
                return error_template.into_response();
            }
        };

        match field.name() {
            Some("file") => {
                filename = field.file_name().unwrap_or("unknown").to_string();
                file_data = match field.bytes().await {
                    Ok(d) => Some(d),
                    Err(e) => {
                        let error_template = ErrorTemplate {
                            error_message: format!("Failed to read uploaded file: {}", e),
                        };
                        return error_template.into_response();
                    }
                };
            }
            Some("name_column") => {
                name_column = match field.text().await {
                    Ok(t) => Some(t),
                    Err(e) => {
                        let error_template = ErrorTemplate {
                            error_message: format!("Failed to read name column field: {}", e),
                        };
                        return error_template.into_response();
                    }
                };
            }
            _ => {}
        }
    }

    // Validate inputs
    let file_data = match file_data {
        Some(d) => d,
        None => {
            let error_template = ErrorTemplate {
                error_message: "No file was uploaded. Please select an Excel file and try again.".to_string(),
            };
            return error_template.into_response();
        }
    };

    let name_column = match name_column {
        Some(c) if !c.is_empty() => c,
        _ => {
            let error_template = ErrorTemplate {
                error_message: "Please specify the name of the column containing names to split.".to_string(),
            };
            return error_template.into_response();
        }
    };

    // Save to temporary file
    let temp_file = match NamedTempFile::new() {
        Ok(f) => f,
        Err(e) => {
            let error_template = ErrorTemplate {
                error_message: format!("Failed to create temporary file: {}", e),
            };
            return error_template.into_response();
        }
    };
    let (mut file, path) = temp_file.into_parts();

    if let Err(e) = tokio::task::block_in_place(|| {
        std::io::Write::write_all(&mut file, &file_data)
    }) {
        let error_template = ErrorTemplate {
            error_message: format!("Failed to save file: {}", e),
        };
        return error_template.into_response();
    }

    // Process the file
    match process_name_splitting(path.to_str().unwrap(), &filename, &name_column) {
        Ok(csv_data) => {
            (
                [
                    ("Content-Type", "text/csv"),
                    ("Content-Disposition", "attachment; filename=\"names_split.csv\""),
                ],
                csv_data,
            )
                .into_response()
        }
        Err(e) => {
            let error_template = ErrorTemplate {
                error_message: e,
            };
            error_template.into_response()
        }
    }
}

fn process_name_splitting(file_path: &str, filename: &str, name_column: &str) -> Result<String, String> {
    // Open the workbook
    let mut workbook = match open_workbook_auto(file_path) {
        Ok(wb) => wb,
        Err(e) => {
            return Err(format!(
                "Failed to open '{}' as an Excel file: {}\n\n\
                Please ensure you're uploading a valid Excel workbook (.xlsx or .xls).",
                filename, e
            ));
        }
    };

    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err("The workbook contains no sheets.".to_string());
    }

    let mut all_output_rows = Vec::new();
    let mut total_rows = 0;

    // Process each sheet
    for sheet_name in &sheet_names {
        let sheet = match workbook.worksheet_range(sheet_name) {
            Ok(s) => s,
            Err(e) => {
                return Err(format!("Failed to read sheet '{}': {}", sheet_name, e));
            }
        };

        let mut rows = sheet.rows();

        // Get header row
        let headers = match rows.next() {
            Some(row) => row.iter().map(|cell| data_to_string(cell)).collect::<Vec<String>>(),
            None => continue, // Skip empty sheets
        };

        // Find the name column index
        let name_col_idx = match headers.iter().position(|h| h == name_column) {
            Some(idx) => idx,
            None => {
                return Err(format!(
                    "Column '{}' not found in sheet '{}'.\n\n\
                    Available columns:\n{}",
                    name_column,
                    sheet_name,
                    headers.iter()
                        .map(|h| format!("  • {}", h))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }
        };

        // Build output header (original columns + new algorithm columns)
        let mut output_header = headers.clone();
        let algorithms = get_algorithms();
        for algo in &algorithms {
            output_header.push(algo.first_name_column.clone());
            output_header.push(algo.last_name_column.clone());
            output_header.push(algo.salutation_column.clone());
        }

        // Only write the header once
        if all_output_rows.is_empty() {
            all_output_rows.push(output_header);
        }

        // Process data rows
        for row in rows {
            let mut output_row: Vec<String> = row.iter().map(|cell| data_to_string(cell)).collect();

            // Get the name value
            let name_value = if name_col_idx < output_row.len() {
                output_row[name_col_idx].clone()
            } else {
                String::new()
            };

            // Apply all algorithms
            let results = apply_all_algorithms(&name_value);
            for (_, result) in results {
                output_row.push(result.first_name);
                output_row.push(result.last_name);
                output_row.push(result.salutation.unwrap_or_default());
            }

            all_output_rows.push(output_row);
            total_rows += 1;
        }
    }

    if total_rows == 0 {
        return Err(format!(
            "No data rows found in any sheet of '{}'.",
            filename
        ));
    }

    // Write to CSV
    let mut csv_output = Vec::new();
    {
        let mut wtr = Writer::from_writer(&mut csv_output);

        for row in all_output_rows {
            if let Err(e) = wtr.write_record(&row) {
                return Err(format!("Failed to write CSV row: {}", e));
            }
        }

        if let Err(e) = wtr.flush() {
            return Err(format!("Failed to finalize CSV output: {}", e));
        }
    }

    let csv_data = String::from_utf8(csv_output).map_err(|e| {
        format!("Failed to encode CSV as UTF-8: {}", e)
    })?;

    Ok(csv_data)
}

async fn email_name_dedup_page() -> EmailNameDedupTemplate {
    EmailNameDedupTemplate {}
}

async fn process_email_name_dedup(
    State(storage): State<CsvStorage>,
    mut multipart: Multipart,
) -> Response {
    // Parse multipart form data
    let mut file_data: Option<bytes::Bytes> = None;
    let mut filename = String::from("unknown");

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => {
                let error_template = ErrorTemplate {
                    error_message: format!("Failed to parse form data: {}", e),
                };
                return error_template.into_response();
            }
        };

        if field.name() == Some("file") {
            filename = field.file_name().unwrap_or("unknown").to_string();
            file_data = match field.bytes().await {
                Ok(d) => Some(d),
                Err(e) => {
                    let error_template = ErrorTemplate {
                        error_message: format!("Failed to read uploaded file: {}", e),
                    };
                    return error_template.into_response();
                }
            };
        }
    }

    // Validate input
    let file_data = match file_data {
        Some(d) => d,
        None => {
            let error_template = ErrorTemplate {
                error_message: "No file was uploaded. Please select an Excel file and try again.".to_string(),
            };
            return error_template.into_response();
        }
    };

    // Save to temporary file
    let temp_file = match NamedTempFile::new() {
        Ok(f) => f,
        Err(e) => {
            let error_template = ErrorTemplate {
                error_message: format!("Failed to create temporary file: {}", e),
            };
            return error_template.into_response();
        }
    };
    let (mut file, path) = temp_file.into_parts();

    if let Err(e) = tokio::task::block_in_place(|| {
        std::io::Write::write_all(&mut file, &file_data)
    }) {
        let error_template = ErrorTemplate {
            error_message: format!("Failed to save file: {}", e),
        };
        return error_template.into_response();
    }

    // Process the file
    match process_xlsx_deduplication(path.to_str().unwrap(), &filename) {
        Ok((xlsx_bytes, log, total_sheets, total_before, total_after, total_dupes)) => {
            let session_id = storage.store_xlsx(xlsx_bytes, log.clone());
            let success_template = EmailNameDedupSuccessTemplate {
                session_id,
                total_sheets,
                total_rows_before: total_before,
                total_rows_after: total_after,
                total_duplicates_removed: total_dupes,
                deduplication_log: log,
            };
            success_template.into_response()
        }
        Err(e) => {
            let error_template = ErrorTemplate {
                error_message: e,
            };
            error_template.into_response()
        }
    }
}

fn process_xlsx_deduplication(
    file_path: &str,
    filename: &str,
) -> Result<(Vec<u8>, String, usize, usize, usize, usize), String> {
    // Read all sheets from the XLSX file
    let sheets_data = read_all_sheets(file_path)
        .map_err(|e| format!("Failed to read '{}': {}", filename, e))?;

    if sheets_data.is_empty() {
        return Err(format!("No sheets with data found in '{}'", filename));
    }

    let mut combined_log = String::new();
    combined_log.push_str("=== EMAIL-THEN-NAME DEDUPLICATION LOG ===\n\n");

    let mut deduplicated_sheets = Vec::new();
    let mut total_before = 0;
    let mut total_after = 0;
    let total_sheets = sheets_data.len();

    for (sheet_name, headers, rows) in sheets_data {
        combined_log.push_str(&format!("\n--- Sheet: {} ---\n", sheet_name));

        let (deduped_rows, sheet_log, before, _removed) = deduplicate_sheet_rows(&headers, rows);

        total_before += before;
        total_after += deduped_rows.len();

        combined_log.push_str(&sheet_log);

        deduplicated_sheets.push((sheet_name, headers, deduped_rows));
    }

    let total_dupes = total_before - total_after;

    combined_log.push_str(&format!(
        "\n=== OVERALL SUMMARY ===\n\
         Total sheets processed: {}\n\
         Total rows before: {}\n\
         Total rows after: {}\n\
         Total duplicates removed: {}\n",
        total_sheets, total_before, total_after, total_dupes
    ));

    // Write deduplicated data to XLSX
    let xlsx_bytes = write_xlsx_to_bytes(deduplicated_sheets)
        .map_err(|e| format!("Failed to create output XLSX: {}", e))?;

    Ok((xlsx_bytes, combined_log, total_sheets, total_before, total_after, total_dupes))
}

async fn download_xlsx(
    State(storage): State<CsvStorage>,
    Path(session_id): Path<String>,
) -> Response {
    match storage.retrieve_xlsx(&session_id) {
        Some(xlsx_data) => {
            (
                [
                    ("Content-Type", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
                    ("Content-Disposition", "attachment; filename=\"deduplicated.xlsx\""),
                ],
                xlsx_data,
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

async fn download_xlsx_log(
    State(storage): State<CsvStorage>,
    Path(session_id): Path<String>,
) -> Response {
    match storage.retrieve_xlsx_log(&session_id) {
        Some(log_data) => {
            (
                [
                    ("Content-Type", "text/plain; charset=utf-8"),
                    ("Content-Disposition", "attachment; filename=\"deduplication_log.txt\""),
                ],
                log_data,
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
