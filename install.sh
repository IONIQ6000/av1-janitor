#!/bin/bash
# Installation script for AV1 Re-encoding Daemon
# This script installs the daemon, TUI, configuration, and systemd service

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Print functions
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    print_error "This script must be run as root (use sudo)"
    exit 1
fi

print_info "Starting AV1 Re-encoding Daemon installation..."

# ============================================================================
# 1. Create system user
# ============================================================================
print_info "Creating system user 'av1d'..."
if id "av1d" &>/dev/null; then
    print_warn "User 'av1d' already exists, skipping creation"
else
    useradd -r -s /bin/false -u 1000 av1d
    print_info "User 'av1d' created"
fi

# ============================================================================
# 2. Create necessary directories
# ============================================================================
print_info "Creating directories..."
mkdir -p /etc/av1d
mkdir -p /var/lib/av1d/jobs
mkdir -p /var/lib/av1d/temp
mkdir -p /media

print_info "Directories created"

# ============================================================================
# 3. Install binaries
# ============================================================================
print_info "Installing binaries..."

# Check if binaries exist in current directory or target/release
if [ -f "./target/release/av1d" ]; then
    cp ./target/release/av1d /usr/local/bin/av1d
    cp ./target/release/av1top /usr/local/bin/av1top
elif [ -f "./av1d" ]; then
    cp ./av1d /usr/local/bin/av1d
    cp ./av1top /usr/local/bin/av1top
else
    print_error "Binaries not found. Please build the project first with: cargo build --release"
    exit 1
fi

chmod +x /usr/local/bin/av1d
chmod +x /usr/local/bin/av1top

print_info "Binaries installed to /usr/local/bin/"

# ============================================================================
# 4. Install configuration file
# ============================================================================
print_info "Installing configuration file..."

if [ -f "/etc/av1d/config.toml" ]; then
    print_warn "Configuration file already exists at /etc/av1d/config.toml"
    print_warn "Backing up to /etc/av1d/config.toml.backup"
    cp /etc/av1d/config.toml /etc/av1d/config.toml.backup
fi

if [ -f "./config.toml" ]; then
    cp ./config.toml /etc/av1d/config.toml
    print_info "Configuration file installed to /etc/av1d/config.toml"
else
    print_error "config.toml not found in current directory"
    exit 1
fi

# ============================================================================
# 5. Set permissions
# ============================================================================
print_info "Setting permissions..."
chown -R av1d:av1d /var/lib/av1d
chmod 755 /var/lib/av1d
chmod 755 /var/lib/av1d/jobs
chmod 755 /var/lib/av1d/temp

# Configuration should be readable by av1d user
chown root:av1d /etc/av1d/config.toml
chmod 640 /etc/av1d/config.toml

print_info "Permissions set"

# ============================================================================
# 6. Install systemd service
# ============================================================================
print_info "Installing systemd service..."

if [ -f "./av1d.service" ]; then
    cp ./av1d.service /etc/systemd/system/av1d.service
    chmod 644 /etc/systemd/system/av1d.service
    print_info "Systemd service file installed"
else
    print_error "av1d.service not found in current directory"
    exit 1
fi

# Reload systemd daemon
print_info "Reloading systemd daemon..."
systemctl daemon-reload

print_info "Systemd service installed"

# ============================================================================
# 7. Verify ffmpeg installation
# ============================================================================
print_info "Verifying ffmpeg installation..."

if ! command -v ffmpeg &> /dev/null; then
    print_error "ffmpeg is not installed. Please install ffmpeg >= 8.0"
    print_error "On Debian/Ubuntu: sudo apt-get install ffmpeg"
    exit 1
fi

FFMPEG_VERSION=$(ffmpeg -version | head -n1 | grep -oP 'ffmpeg version \K[0-9]+' || echo "0")
if [ "$FFMPEG_VERSION" -lt 8 ]; then
    print_error "ffmpeg version $FFMPEG_VERSION is too old. Version 8.0 or higher is required."
    exit 1
fi

print_info "ffmpeg version $FFMPEG_VERSION detected (OK)"

# ============================================================================
# 8. Installation complete
# ============================================================================
echo ""
print_info "============================================"
print_info "Installation completed successfully!"
print_info "============================================"
echo ""
print_info "Next steps:"
echo ""
echo "  1. Edit configuration (if needed):"
echo "     sudo nano /etc/av1d/config.toml"
echo ""
echo "  2. Enable daemon to start on boot:"
echo "     sudo systemctl enable av1d"
echo ""
echo "  3. Start the daemon:"
echo "     sudo systemctl start av1d"
echo ""
echo "  4. Check daemon status:"
echo "     sudo systemctl status av1d"
echo ""
echo "  5. View daemon logs:"
echo "     sudo journalctl -u av1d -f"
echo ""
echo "  6. Monitor encoding jobs with TUI:"
echo "     av1top"
echo ""
print_info "Configuration file: /etc/av1d/config.toml"
print_info "Job state directory: /var/lib/av1d/jobs"
print_info "Temp output directory: /var/lib/av1d/temp"
echo ""
