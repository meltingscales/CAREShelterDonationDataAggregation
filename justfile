# List available recipes
help:
    @just --list

# Build the project in release mode
build:
    cargo build --release

# Run all tests
test:
    cargo test

# Run the web server locally
run:
    cargo run --bin web

# Run the web server in release mode (faster)
run-release:
    cargo run --release --bin web

# Run the field name printer
run-print-field-names:
    cargo run --bin print-donorsnap-field-names

# Run the field mapper to extract DonorSnap data to CSV
run-map-fields:
    cargo run --bin map-fields

# Format Rust code
fmt:
    cargo fmt

# Show version info that will be embedded in the build
version-info:
    @echo "Git Commit:  $(git rev-parse --short HEAD)"
    @echo "Git Branch:  $(git rev-parse --abbrev-ref HEAD)"
    @echo "Build Date:  $(git log -1 --format=%ci)"

# Clean up generated files and build artifacts
clean:
    rm -rf .ropeproject/
    cargo clean

# Security Scanning
# =================

# Run Trivy security scan on the Docker image
trivy-scan:
    #!/usr/bin/env bash
    echo "Building Docker image for scanning..."
    docker build -t care-shelter-donation-aggregation:scan .
    echo ""
    echo "Running Trivy vulnerability scan..."
    trivy image --severity HIGH,CRITICAL care-shelter-donation-aggregation:scan

# Run Trivy scan with all severity levels
trivy-scan-all:
    #!/usr/bin/env bash
    echo "Building Docker image for scanning..."
    docker build -t care-shelter-donation-aggregation:scan .
    echo ""
    echo "Running Trivy vulnerability scan (all severities)..."
    trivy image care-shelter-donation-aggregation:scan

# Run Trivy scan and save report to file
trivy-scan-report:
    #!/usr/bin/env bash
    echo "Building Docker image for scanning..."
    docker build -t care-shelter-donation-aggregation:scan .
    echo ""
    echo "Running Trivy vulnerability scan and saving report..."
    trivy image --severity HIGH,CRITICAL --format json --output trivy-report.json care-shelter-donation-aggregation:scan
    trivy image --severity HIGH,CRITICAL --format table --output trivy-report.txt care-shelter-donation-aggregation:scan
    echo "Reports saved to trivy-report.json and trivy-report.txt"

# Docker operations
# ================

# Build Docker image
docker-build:
    docker build -t care-shelter-donation-aggregation:latest .

# Build Docker image with a specific tag
docker-build-tag tag:
    docker build -t care-shelter-donation-aggregation:{{tag}} .

# Run Docker container locally
docker-run port="8080":
    docker run -p {{port}}:8080 care-shelter-donation-aggregation:latest

# Stop all running containers for this project
docker-stop:
    docker ps -q --filter ancestor=care-shelter-donation-aggregation:latest | xargs -r docker stop

# Remove Docker image
docker-clean:
    docker rmi care-shelter-donation-aggregation:latest

# GCP Deployment
# ==============

# Set these variables for your GCP project
GCP_PROJECT := env_var_or_default("GCP_PROJECT", "careshelterdonationdataagg")
GCP_REGION := env_var_or_default("GCP_REGION", "us-central1")
SERVICE_NAME := "care-shelter-donation-aggregation"
DOMAIN_NAME := env_var_or_default("DOMAIN_NAME", "meltingscales-donorsnap-care.example.com")

# Build and push Docker image to Google Container Registry
gcp-push:
    docker build -t gcr.io/{{GCP_PROJECT}}/{{SERVICE_NAME}}:latest .
    docker push gcr.io/{{GCP_PROJECT}}/{{SERVICE_NAME}}:latest

# Build and push Docker image with a specific tag
gcp-push-tag tag:
    docker build -t gcr.io/{{GCP_PROJECT}}/{{SERVICE_NAME}}:{{tag}} .
    docker push gcr.io/{{GCP_PROJECT}}/{{SERVICE_NAME}}:{{tag}}

# Deploy to Google Cloud Run
gcp-deploy:
    gcloud run deploy {{SERVICE_NAME}} \
        --image gcr.io/{{GCP_PROJECT}}/{{SERVICE_NAME}}:latest \
        --platform managed \
        --region {{GCP_REGION}} \
        --allow-unauthenticated \
        --port 8080 \
        --project {{GCP_PROJECT}}

# Deploy a specific tagged version to Cloud Run
gcp-deploy-tag tag:
    gcloud run deploy {{SERVICE_NAME}} \
        --image gcr.io/{{GCP_PROJECT}}/{{SERVICE_NAME}}:{{tag}} \
        --platform managed \
        --region {{GCP_REGION}} \
        --allow-unauthenticated \
        --port 8080 \
        --project {{GCP_PROJECT}}

# Build, push, and deploy to GCP in one command
gcp-deploy-all:
    just gcp-push
    just gcp-deploy

# View Cloud Run service logs
gcp-logs:
    gcloud run services logs read {{SERVICE_NAME}} --region {{GCP_REGION}} --project {{GCP_PROJECT}}

# Get Cloud Run service URL
gcp-url:
    gcloud run services describe {{SERVICE_NAME}} --region {{GCP_REGION}} --project {{GCP_PROJECT}} --format 'value(status.url)'

# Domain Management
# =================

# Map a custom domain to the Cloud Run service
gcp-domain-map domain=DOMAIN_NAME:
    gcloud run domain-mappings create \
        --service {{SERVICE_NAME}} \
        --domain {{domain}} \
        --region {{GCP_REGION}} \
        --project {{GCP_PROJECT}}

# List all domain mappings
gcp-domain-list:
    gcloud run domain-mappings list --region {{GCP_REGION}} --project {{GCP_PROJECT}}

# Get DNS records needed for domain verification
gcp-domain-records domain=DOMAIN_NAME:
    gcloud run domain-mappings describe {{domain}} --region {{GCP_REGION}} --project {{GCP_PROJECT}}

# Delete a domain mapping
gcp-domain-delete domain=DOMAIN_NAME:
    gcloud run domain-mappings delete {{domain}} --region {{GCP_REGION}} --project {{GCP_PROJECT}}

# Systemd Service Setup (for GCP VM)
# ====================================

# Install as systemd service running on port 3003
# Run with sudo
systemd-install:
    #!/usr/bin/env bash
    set -euo pipefail

    if [[ $EUID -ne 0 ]]; then
        echo "Error: This recipe must be run as root (use sudo)."
        exit 1
    fi

    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    SERVICE_NAME="care-shelter-donation"
    PORT="${PORT:-3003}"
    REPO_DIR="${REPO_DIR:-${SCRIPT_DIR}}"
    USER="${SUDO_USER:-root}"

    echo "Installing systemd service: ${SERVICE_NAME}"

    # Build release binary first (skip if already built)
    if [[ ! -f "${REPO_DIR}/target/release/web" ]]; then
        echo "Building release binary..."
        (cd "${REPO_DIR}" && cargo build --release)
    else
        echo "Binary already exists, skipping build."
    fi

    # Copy and template service file
    sed -e "s|USER_PLACEHOLDER|${USER}|g" \
        -e "s|REPO_DIR_PLACEHOLDER|${REPO_DIR}|g" \
        "${SCRIPT_DIR}/systemd/${SERVICE_NAME}.service" \
        > /etc/systemd/system/${SERVICE_NAME}.service

    # Reload systemd and enable service
    systemctl daemon-reload
    systemctl enable ${SERVICE_NAME}
    systemctl restart ${SERVICE_NAME}

    echo "Service installed and started on port ${PORT}!"
    echo ""
    echo "Commands:"
    echo "  sudo systemctl status ${SERVICE_NAME}"
    echo "  sudo systemctl restart ${SERVICE_NAME}"
    echo "  sudo journalctl -u ${SERVICE_NAME} -f"

# Uninstall systemd service
# Run with sudo
systemd-uninstall:
    #!/usr/bin/env bash
    SERVICE_NAME="care-shelter-donation"

    if [[ $EUID -ne 0 ]]; then
        echo "Error: This recipe must be run as root (use sudo)."
        exit 1
    fi

    echo "Stopping and disabling ${SERVICE_NAME}..."
    systemctl stop ${SERVICE_NAME} 2>/dev/null || true
    systemctl disable ${SERVICE_NAME} 2>/dev/null || true
    rm -f /etc/systemd/system/${SERVICE_NAME}.service
    systemctl daemon-reload
    echo "Service uninstalled."

# Show service status
systemd-status:
    #!/usr/bin/env bash
    SERVICE_NAME="${SERVICE_NAME:-care-shelter-donation}"
    systemctl status ${SERVICE_NAME}

# View service logs
systemd-logs:
    #!/usr/bin/env bash
    SERVICE_NAME="${SERVICE_NAME:-care-shelter-donation}"
    journalctl -u ${SERVICE_NAME} -f

# Restart the service
systemd-restart:
    #!/usr/bin/env bash
    SERVICE_NAME="${SERVICE_NAME:-care-shelter-donation}"
    systemctl restart ${SERVICE_NAME}
    systemctl status ${SERVICE_NAME}
