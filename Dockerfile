# Multi-stage build for AV1 Re-encoding Daemon
# Stage 1: Build the Rust binaries
FROM rust:1.75-bookworm as builder

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binaries
RUN cargo build --release

# Stage 2: Create minimal runtime image
FROM debian:bookworm-slim

# Install ffmpeg system dependency
RUN apt-get update && apt-get install -y \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Verify ffmpeg version (should be >= 8.0 for Debian Bookworm)
RUN ffmpeg -version

# Create system user for daemon
RUN useradd -r -s /bin/false -u 1000 av1d

# Create necessary directories
RUN mkdir -p /etc/av1d \
    /var/lib/av1d/jobs \
    /var/lib/av1d/temp \
    /media

# Copy compiled binaries from builder stage
COPY --from=builder /build/target/release/av1d /usr/local/bin/av1d
COPY --from=builder /build/target/release/av1top /usr/local/bin/av1top

# Make binaries executable
RUN chmod +x /usr/local/bin/av1d /usr/local/bin/av1top

# Set ownership of data directories
RUN chown -R av1d:av1d /var/lib/av1d

# Switch to non-root user
USER av1d

# Set working directory
WORKDIR /var/lib/av1d

# Default command runs the daemon
CMD ["/usr/local/bin/av1d", "--config", "/etc/av1d/config.toml"]
