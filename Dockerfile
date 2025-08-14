# Multi-stage build for optimal image size
FROM rust:1.89-slim as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install ca-certificates for HTTPS requests
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 watchlistarr

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/watchlistarr /app/watchlistarr

# Change ownership
RUN chown watchlistarr:watchlistarr /app/watchlistarr

USER watchlistarr

# Expose any ports if needed (currently none)
# EXPOSE 8080

CMD ["./watchlistarr"]