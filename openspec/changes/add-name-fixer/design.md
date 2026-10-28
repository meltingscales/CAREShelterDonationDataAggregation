# Design

## Context

The closest existing tool is the Name Splitter (`src/name_splitter.rs` + `process_name_splitter` / `name_splitter_process_api` in `src/main.rs`, template `templates/name_splitter.html`). It accepts Excel only via `calamine::open_workbook_auto` on a temp file, takes a column name as free text, iterates all sheets, and writes CSV with the `csv` crate. Its page handler and API handler each duplicate ~100 lines of multipart parsing. Requirements are in `specs/name-capitalization/spec.md`.

## Goals / Non-Goals

**Goals:**
- Pure, unit-tested capitalization logic independent of HTTP and file formats.
- One shared form-parsing + processing path used by both the HTML and JSON handlers.
- CSV and Excel input with the same downstream code.

**Non-Goals:**
- Particle handling (`van`, `de`, `la`) or `Mac` prefixes — both are ambiguous (`De La Cruz` vs `de la Cruz`, `Mack`/`Macy`); left to the single-case-only option and manual review.
- XLSX output, in-browser preview, or column auto-detection.
- Refactoring the Name Splitter's duplicated handlers.

## Decisions

**1. Module `src/name_fixer.rs` owns styles and the algorithm.**
`enum NameFixStyle { ProperName, TitleCase, Uppercase, Lowercase }` with `from_id(&str)`, plus `struct NameFixStyleInfo { id, name, description, example_before, example_after }` (Serialize + ToSchema) returned by `get_styles()` — mirrors `get_algorithms()` / `NameSplitAlgorithm` so the template and `/api/name-fixer/styles` render from one source. Entry point: `fix_value(value: &str, style, only_single_case: bool) -> String`.

**2. Proper Name algorithm (char scan, no regex).**
Lowercase the whole value, then walk chars, uppercasing a letter at start-of-string or when the previous char is not alphanumeric (space, `-`, `'`, `.`, `(` …), so nicknames like `(JACKIE)` and initials like `J.R.` come out right. Post-pass per whitespace word: if it starts with `Mc` and has a third letter, uppercase that letter; if the word (stripped of trailing `.`/`,`) is in {II, III, IV, VI, VII, VIII, IX, X}, uppercase it entirely. A char scan keeps whitespace and punctuation byte-for-byte intact (spec: preserve non-letter content) where split/join on words would collapse runs of spaces. Unicode via `char::to_uppercase`/`to_lowercase` (handles `É`, `Ñ`).
*Alternative:* a crate such as `heck`/`titlecase` — rejected; they target identifiers/English headlines, not surnames, and add a dependency for ~40 lines.

**3. Single-case check.** A value qualifies if it contains at least one letter and every alphabetic char is uppercase, or every one is lowercase. Values with no letters are returned unchanged regardless of style.

**4. Input reading → common `Vec<(sheet_name, headers, rows)>`.**
Format chosen by filename extension: `.csv` → `csv::ReaderBuilder::new().flexible(true)` over the uploaded bytes (strip a leading UTF-8 BOM; non-UTF-8 surfaces as a read error); otherwise Excel via calamine, reusing `xlsx_utils::read_all_sheets` on a `NamedTempFile` exactly as existing tools do. The CSV is treated as one sheet named after the file. Same shape as `read_all_sheets` output so the rest of the pipeline is format-agnostic.
*Alternative:* `open_workbook_auto_from_rs` on an in-memory cursor to skip the temp file — viable but diverges from the established `read_all_sheets` helper; not worth it. Note `read_all_sheets` skips sheets with no data rows, which is acceptable here.

**5. Column matching and output shape.** Requested names are split on `,`, trimmed, empty entries dropped; matched against headers with `trim().to_lowercase()`. A requested column must match in at least one sheet, otherwise error listing all distinct headers. One sheet → output headers unchanged. Multiple sheets → `Source Sheet` + ordered union of headers (first-seen order and spelling; headers are merged using the same trim + case-insensitive comparison as column matching, so `Name` and `name` share one column), each row remapped by header name, blanks for absent columns. Rows shorter than the header are padded with blanks so the cell count is stable.

**6. Shared handler core.**
Reading, normalizing and CSV writing live in `name_fixer.rs` as `process_name_fixing(bytes, filename, columns, style, only_single_case) -> Result<(csv, rows), String>` plus `output_filename()`, so they are unit-tested in the library. `main.rs` keeps only `async fn parse_name_fixer_form(Multipart) -> Result<NameFixerForm, String>` (fields: `file`, `columns`, `style`, `only_single_case` — checkbox present = true; HTML sends it checked by default). `process_name_fixer` (HTML) maps `Err` → `ErrorTemplate`, `Ok` → CSV attachment with `Content-Disposition: attachment; filename="<stem>_names_fixed.csv"` (stem sanitized to `[A-Za-z0-9 _.-]`). `name_fixer_process_api` maps `Err` → 400 JSON, `Ok` → `NameFixerResult { csv_data, total_rows, columns, style }`. Avoids repeating the Name Splitter's duplication.
*Note:* the API is posted with `multipart/form-data` where an absent `only_single_case` would mean false; the API docs state this explicitly. Unknown `style` → error listing valid ids.

**7. Routes / UI.** `GET /name-fixer`, `POST /name-fixer/process`, `GET /api/name-fixer/styles`, `POST /api/name-fixer/process`; register both API paths and schemas in the `#[openapi]` block under tag `Utilities`. Template `name_fixer.html` follows `name_splitter.html` structure (instructions, styles table from `get_styles()`, privacy notice, form with file `accept=".csv,.xls,.xlsx"`, text input for columns, `<select>` for style defaulting to Proper Name, checked checkbox for single-case-only). Navbar link added after Name Splitter in `base.html`.

## Risks / Trade-offs

- [Proper Name miscapitalizes some names (`Macdonald`, `van der Berg`, `DeAndre` when input is all caps)] → Page explanation states limits; single-case-only default protects already-correct mixed-case data; staff can review output.
- [Applying a style to non-name columns (e.g. email) would mangle them] → Only user-listed columns are touched; page tip warns to choose name/address columns only.
- [Large uploads] → Existing `RequestBodyLimitLayer` applies; processing is linear.
- [CSV in non-UTF-8 encodings (Windows-1252 Excel exports)] → Clear error suggesting "Save As CSV UTF-8" or uploading the .xlsx instead.
- [`to_uppercase` can expand chars (`ß` → `SS`) in UPPERCASE style] → Acceptable; matches standard Unicode behavior.

## Migration Plan

Additive only: new routes and template. Deploy via the existing process; rollback by reverting the commit. No data or config migration.
