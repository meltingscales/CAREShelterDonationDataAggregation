# Spec Delta

## Purpose

Lets shelter staff fix inconsistent capitalization (for example ALL CAPS or all lowercase names) in selected columns of a spreadsheet and download the corrected data as CSV.

## ADDED Requirements

### Requirement: Accept CSV and Excel uploads
The Name Fixer SHALL accept an uploaded file in CSV (.csv), Excel 2007+ (.xlsx), or legacy Excel (.xls) format, where the first row of each sheet (or of the CSV) is the header row.

#### Scenario: CSV upload
- **WHEN** a user uploads a UTF-8 `.csv` file with a header row and data rows
- **THEN** the file is processed and a normalized CSV is returned

#### Scenario: CSV with byte-order mark
- **WHEN** a user uploads a UTF-8 CSV that begins with a byte-order mark
- **THEN** the first header name is matched without the byte-order mark

#### Scenario: Excel upload
- **WHEN** a user uploads an `.xlsx` or `.xls` workbook
- **THEN** the workbook is processed and a normalized CSV is returned

#### Scenario: Unreadable file
- **WHEN** a user uploads a file that is not valid CSV or Excel, or a CSV that is not UTF-8
- **THEN** the system shows an error explaining the file could not be read and no download is produced

### Requirement: Select columns to normalize
The user SHALL specify one or more column names, comma-separated. Column names SHALL match headers ignoring surrounding whitespace and letter case. Only the selected columns SHALL be modified.

#### Scenario: Multiple columns selected
- **WHEN** the user enters `First Name, last name` and the file has headers `First Name` and `Last Name`
- **THEN** values in both columns are normalized and all other columns are unchanged

#### Scenario: Column not found
- **WHEN** none of the file's sheets contain a header matching a requested column name
- **THEN** the system shows an error naming the missing column and listing the available column names

#### Scenario: No columns entered
- **WHEN** the user submits without entering any column name
- **THEN** the system shows an error asking for at least one column

### Requirement: Proper Name style
The Proper Name style SHALL lowercase the value, then uppercase every letter that starts the value or follows a non-alphanumeric character (such as a space, hyphen, apostrophe, period or parenthesis), uppercase the letter after a leading `Mc`, and render Roman-numeral suffix words (II through X) fully uppercase.

#### Scenario: All caps full name
- **WHEN** the value `JANE SMITH` is normalized with Proper Name
- **THEN** the result is `Jane Smith`

#### Scenario: Apostrophes, hyphens and Mc
- **WHEN** the values `O'BRIEN`, `mary-kate smith-jones`, `MCDONALD`, and `JACK (JACKIE) SMITH` are normalized with Proper Name
- **THEN** the results are `O'Brien`, `Mary-Kate Smith-Jones`, `McDonald`, and `Jack (Jackie) Smith`

#### Scenario: Roman numeral suffix
- **WHEN** the value `JOHN SMITH III` is normalized with Proper Name
- **THEN** the result is `John Smith III`

### Requirement: Simple Title Case, UPPERCASE and lowercase styles
The system SHALL offer Simple Title Case (first letter of each whitespace-separated word uppercase, remaining letters lowercase), UPPERCASE (all letters uppercase), and lowercase (all letters lowercase).

#### Scenario: Simple Title Case does not special-case punctuation
- **WHEN** the value `O'BRIEN SMITH-JONES` is normalized with Simple Title Case
- **THEN** the result is `O'brien Smith-jones`

#### Scenario: UPPERCASE and lowercase
- **WHEN** the value `Jane Smith` is normalized with UPPERCASE and with lowercase
- **THEN** the results are `JANE SMITH` and `jane smith` respectively

### Requirement: Only fix single-case values option
The system SHALL provide an option, enabled by default, under which a value is changed only if all of its letters are uppercase or all are lowercase; values containing both uppercase and lowercase letters SHALL be left exactly as-is.

#### Scenario: Mixed-case value preserved
- **WHEN** the option is enabled and the value `DeAndre MacKenzie` is normalized with Proper Name
- **THEN** the value is unchanged

#### Scenario: All-caps value fixed
- **WHEN** the option is enabled and the value `DEANDRE MACKENZIE` is normalized with Proper Name
- **THEN** the result is `Deandre Mackenzie`

#### Scenario: Option disabled
- **WHEN** the option is disabled and the value `jANE sMITH` is normalized with Proper Name
- **THEN** the result is `Jane Smith`

### Requirement: Preserve non-letter content
Normalization SHALL change only letter case. Whitespace, digits, punctuation, empty cells and the number of rows and columns SHALL be preserved.

#### Scenario: Empty and non-alphabetic values
- **WHEN** a selected column contains an empty cell and the value `12345`
- **THEN** both values are output unchanged

#### Scenario: Non-ASCII letters
- **WHEN** the value `JOSÉ PEÑA` is normalized with Proper Name
- **THEN** the result is `José Peña`

### Requirement: CSV output
The system SHALL return a downloadable CSV containing the header row and all data rows with selected columns normalized, named after the uploaded file with a `_names_fixed.csv` suffix. For multi-sheet workbooks, rows from all sheets SHALL be included, with a leading `Source Sheet` column and the union of all sheets' headers.

#### Scenario: Single sheet download
- **WHEN** a user processes `donors.xlsx` containing one sheet
- **THEN** the browser downloads `donors_names_fixed.csv` with the same columns in the same order

#### Scenario: Multi-sheet workbook
- **WHEN** a user processes a workbook with sheets `Adopters` and `Fosters`
- **THEN** the CSV has a first column `Source Sheet` identifying each row's sheet, and columns missing from a sheet are blank for that sheet's rows

### Requirement: Algorithm explanation on page
The Name Fixer page SHALL list each normalization style with a brief plain-English description of its algorithm and at least one before/after example, and SHALL explain the single-case-only option.

#### Scenario: Viewing the page
- **WHEN** a user opens the Name Fixer page
- **THEN** they see each style's description and example, and the upload form with column, style and option inputs

### Requirement: JSON API
The system SHALL expose an endpoint listing the available styles and an endpoint that accepts the same upload inputs and returns the normalized CSV text and row count as JSON, both documented in the OpenAPI specification.

#### Scenario: List styles
- **WHEN** a client requests the styles endpoint
- **THEN** it receives each style's identifier, display name, description and example

#### Scenario: Process via API with error
- **WHEN** a client posts a file to the process endpoint naming a column that does not exist
- **THEN** it receives HTTP 400 with a JSON error message

### Requirement: No data retention
Uploaded files SHALL be processed without being stored beyond the request, consistent with the application's other tools.

#### Scenario: After processing
- **WHEN** a Name Fixer request completes, successfully or with an error
- **THEN** no copy of the uploaded file or its output remains on the server
