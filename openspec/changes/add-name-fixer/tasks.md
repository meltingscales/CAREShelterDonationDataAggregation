# Tasks

## 1. Capitalization logic (`src/name_fixer.rs`)

- [x] 1.1 Create `src/name_fixer.rs` with GPL header, `NameFixStyle` enum (`from_id`), `NameFixStyleInfo` (Serialize + ToSchema) and `get_styles()` for ProperName, TitleCase, Uppercase, Lowercase with description and before/after example; register module and re-exports in `src/lib.rs`; verify `cargo build` succeeds
- [x] 1.2 Implement `fix_value(value, style, only_single_case)` per design decisions 2–3 (char-scan Proper Name with hyphen/apostrophe/`Mc`/Roman-numeral handling, Simple Title Case, upper, lower, single-case check, no-letter passthrough); verify with unit tests covering every scenario in the spec's Proper Name, Simple Title Case/UPPERCASE/lowercase, single-case option, and preserve-non-letter requirements (including `JOSÉ PEÑA`, double spaces preserved, empty string, `12345`), and that each `get_styles()` example round-trips through `fix_value`; `cargo test name_fixer` passes

## 2. File processing (`src/name_fixer.rs`, form parsing in `src/main.rs`)

- [x] 2.1 Add CSV reading (BOM strip, flexible rows, UTF-8 error message) producing the `(sheet_name, headers, rows)` shape, and Excel reading via `read_all_sheets` on a `NamedTempFile`, selected by file extension; verify with unit tests for a BOM-prefixed CSV and a non-UTF-8 CSV error
- [x] 2.2 Implement `process_name_fixing`: parse comma-separated columns (trim, case-insensitive match), error when none given or a column matches no sheet (listing available headers), normalize only selected cells, single-sheet passthrough vs multi-sheet `Source Sheet` + header union with blank fill, write CSV, return row count; verify with unit tests for multi-column selection, missing column error, empty column input, and a two-sheet workbook with differing headers (build the xlsx in-test with `write_xlsx_to_bytes`)
- [x] 2.3 Implement `parse_name_fixer_form` (file, columns, style, only_single_case checkbox semantics, unknown-style error listing valid ids) and output filename `<sanitized stem>_names_fixed.csv`; verify filename sanitization with a unit test

## 3. HTTP handlers, API and page

- [x] 3.1 Add `name_fixer_page`, `process_name_fixer` (ErrorTemplate on error, CSV attachment on success), `name_fixer_styles_api`, `name_fixer_process_api` (`NameFixerResult` JSON / 400 JSON error), routes, and OpenAPI path + schema registration under `Utilities`; verify `cargo build` and that `/openapi` lists both new endpoints
- [x] 3.2 Create `templates/name_fixer.html` (about/how-it-works, styles table from `get_styles()`, single-case option explanation, limitations note and "only pick name/address columns" tip, privacy notice, form with `.csv,.xls,.xlsx` file input, columns text input, style select defaulting to Proper Name, checked single-case checkbox) and add navbar link after Name Splitter in `templates/base.html`; verify the page renders at `/name-fixer` with `just` / `cargo run --bin web`

## 4. Integration check

- [x] 4.1 Run the server and exercise end to end: upload an all-caps CSV and a multi-sheet xlsx through the page and the API (`curl -F`), confirm downloaded filename, `Source Sheet` column, untouched non-selected columns, mixed-case values preserved with the option on, and a 400 JSON for a bad column; `cargo test` passes in full
