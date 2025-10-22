# Build stage
FROM rust:1.83-slim AS builder

WORKDIR /app

# Install git for build.rs to capture version info
RUN apt-get update && apt-get install -y git && rm -rf /var/lib/apt/lists/*

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Copy build script (for git version info)
COPY build.rs ./

# Copy source code
COPY src ./src
COPY templates ./templates

# Copy .git directory for version info (git commands in build.rs need this)
COPY .git ./.git

# Build the application in release mode
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

# Expose port
EXPOSE 8080

# Run the web server
CMD ["/app/web"]
