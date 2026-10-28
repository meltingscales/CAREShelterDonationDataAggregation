# Proposal

## Why

Shelter staff (requested by Karin) regularly receive donor/adopter exports where names and other text fields are in inconsistent capitalization, most often ALL CAPS (`JANE SMITH`) or all lowercase (`jane smith`). Fixing these by hand in Excel before importing into DonorSnap or sending mailings is slow and error-prone, and no existing tool in the app addresses it.

## What Changes

- Add a new **Name Fixer** page (`/name-fixer`) linked from the navbar, alongside the existing Name Splitter.
- Staff upload a CSV or Excel (.xlsx/.xls) file, enter the column(s) to normalize, pick a normalization style, and download a CSV where only those columns have been re-capitalized; all other data is passed through unchanged.
- Normalization styles offered:
  - **Proper Name** (default) — capitalizes each word and the part after hyphens and apostrophes, handles `Mc` prefixes and Roman-numeral suffixes (`O'BRIEN` → `O'Brien`, `MCDONALD` → `McDonald`, `SMITH-JONES` → `Smith-Jones`, `JOHN SMITH III` → `John Smith III`).
  - **Simple Title Case** — first letter of each whitespace-separated word upper, rest lower.
  - **UPPERCASE** and **lowercase**.
- An option, on by default, to **only change values that are entirely uppercase or entirely lowercase**, so already-mixed-case names such as `DeAndre` or `MacKenzie` are left alone.
- The page shows a short plain-English explanation of each style and the algorithm used, with examples.
- Matching JSON API endpoints under `/api/name-fixer/` documented in the existing OpenAPI/Swagger UI, mirroring the Name Splitter.
- Same privacy guarantee as other tools: files processed in memory/temp files and not retained.

## Capabilities

### New Capabilities
- `name-capitalization`: Re-capitalizing selected text columns of an uploaded spreadsheet according to a chosen normalization style and returning the result as CSV.

### Modified Capabilities
<!-- None: no existing specs in openspec/specs/. -->

## Impact

- New module `src/name_fixer.rs` (pure capitalization logic + unit tests), re-exported from `src/lib.rs`.
- `src/main.rs`: new page/process handlers, API handlers, routes, OpenAPI registration; CSV input parsing (CSV input is new for upload tools — existing tools accept Excel only).
- New template `templates/name_fixer.html`; navbar link in `templates/base.html`.
- No new dependencies (`csv`, `calamine`, `regex` already present). No changes to existing tools' behavior.
