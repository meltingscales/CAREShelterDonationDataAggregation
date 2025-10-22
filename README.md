# C.A.R.E. Shelter Donation Data Aggregation

https://care-shelter-donation-aggregation-163012697625.us-central1.run.app/

A web application to aggregate donation data from multiple sources into a standardized CSV format.

## What is this?

C.A.R.E. Shelter receives donations through multiple platforms (Qgiv, PayPal, Square, ShelterLuv, and manual entries). Each platform exports data with different field names and formats, making it difficult to import all donations into DonorSnap for centralized donor management.

This tool solves that problem by:
- **Accepting** a single Excel file containing sheets from different donation sources
- **Mapping** each source's fields to DonorSnap's expected field names
- **Combining** all donation records into one standardized CSV file
- **Simplifying** the import process into DonorSnap's Excel Import feature

Instead of manually reformatting data from each platform, you can now upload one consolidated spreadsheet and get back a DonorSnap-ready CSV file with consistent field names for donor information (name, email, address), donation details (amount, date, payment method), and notes.

## Features

- **Web Interface**: Upload Excel spreadsheets and download aggregated CSV data
- **Data-Driven Mappings**: Field mappings are defined in Rust code and displayed on the web
- **Multiple Source Support**: Handles data from DonorSnap, Qgiv, ShelterLuv, Square, and Facebook PayPal
- **Privacy First**: No data retention - files are processed in memory and immediately deleted

## Privacy & Security

**Your data is never stored.** This tool processes uploaded files in real-time and automatically deletes them:

- Files are uploaded to temporary memory using `NamedTempFile` which automatically deletes files when they go out of scope
- No database exists - there is no mechanism to store data
- The CSV output is generated in memory and streamed directly to your browser
- No logs are kept of file contents or donor information
- All processing happens server-side without third-party services

**Open Source**: The entire codebase is open source so you can verify these privacy claims. You can also run the tool locally on your own computer for complete control over your data.

## Running the Website

### Prerequisites

- Rust and Cargo installed
- Just (command runner) - optional but recommended

### Using Just

```bash
# Run the web server in development mode
just run

# Run the web server in release mode (faster, recommended for production)
just run-release
```

### Using Cargo Directly

```bash
# Development mode
cargo run --bin web

# Release mode
cargo run --release --bin web
```

The server will start at `http://localhost:8080`.

## Using the Web Application

1. Navigate to `http://localhost:8080` in your browser
2. Upload an Excel file (.xls or .xlsx) containing your donation data
3. The file must contain sheets with these exact names:
   - DonorSnap
   - Qgiv
   - ShelterLuv
   - Square
   - Facebook PayPal
4. Each sheet should have headers matching the field names for that source
5. The application will process all sheets and download a combined CSV file

## Additional Pages

- **About** - `http://localhost:8080/about` - Learn more about why this tool exists and how it works
- **Mappings** - `http://localhost:8080/mappings` - View how fields from each source are mapped to the standardized output
- **FAQ** - `http://localhost:8080/faq` - Frequently asked questions about using the tool

## CLI Tools

The project also includes command-line tools:

```bash
# Extract data from a spreadsheet to output.csv
just run-map-fields

# Print DonorSnap field names
just run-print-field-names
```

## Standardized Output Fields

The output CSV contains these fields:

- First, Last, Company
- EMail, Phone
- Address1, Address2, Address3, City, State/Province, Zip/Postal Code, Country
- Salutation
- Donation Date, Amount, Donation Type, Payment Method
- DonationNote

## Development

```bash
# Format code
just fmt

# Build release binaries
just build

# Clean build artifacts
just clean
```

## Architecture

- **src/lib.rs**: Data-driven mapping definitions shared between CLI and web
- **src/main.rs**: Web server using Axum framework
- **src/map-fields.rs**: CLI tool for batch processing
- **templates/**: HTML templates for the web interface

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

**Copyright (C) 2025 Henry Post**

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
