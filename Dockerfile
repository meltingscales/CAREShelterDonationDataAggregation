# Build stage
FROM rust:1.83-slim AS builder

WORKDIR /app

# Install git for build.rs to capture version info
RUN apt-get update && apt-get install -y git && rm -rf /var/lib/apt/lists/*

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Copy build script (for git version info)
COPY build.rs ./

# Create dummy source files to build dependencies
RUN mkdir -p src && \
    echo "pub mod data_mappings; pub mod normalization; pub mod deduplication;" > src/lib.rs && \
    echo "pub fn dummy() {}" > src/data_mappings.rs && \
    echo "pub fn dummy() {}" > src/normalization.rs && \
    echo "pub fn dummy() {}" > src/deduplication.rs && \
    echo "fn main() {}" > src/main.rs && \
    mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/print-donorsnap-field-names.rs && \
    echo "fn main() {}" > src/bin/map-fields.rs && \
    mkdir -p templates && \
    echo "" > templates/dummy.html

# Create dummy .git directory for build.rs
RUN git init && \
    git config user.email "docker@example.com" && \
    git config user.name "Docker Build" && \
    git add -A && \
    git commit -m "dummy"

# Build dependencies only - this layer will be cached
RUN cargo build --release --bin web

# Remove dummy source, templates, and git
RUN rm -rf src templates .git

# Copy actual source code
COPY src ./src
COPY templates ./templates

# Copy .git directory for version info (git commands in build.rs need this)
COPY .git ./.git

# Build the application with real source code
# This will be fast because dependencies are already compiled
RUN cargo build --release --bin web

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/web /app/web

# Copy templates directory
COPY --from=builder /app/templates /app/templates

# Copy static directory (CSS, etc.)
COPY static /app/static

# Expose port
EXPOSE 8080

# Run the web server
CMD ["/app/web"]
