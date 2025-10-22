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
