#!/bin/bash
# Update script for AV1 Re-encoding Daemon
# Stops service, pulls latest code, rebuilds, and restarts

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    print_error "This script must be run as root (use sudo)"
    exit 1
fi

print_info "Stopping av1d service..."
systemctl stop av1d

print_info "Pulling latest code from git..."
git pull

print_info "Building release binaries..."
cargo build --release

print_info "Installing binaries..."
cp ./target/release/av1d /usr/local/bin/av1d
cp ./target/release/av1top /usr/local/bin/av1top
chmod +x /usr/local/bin/av1d
chmod +x /usr/local/bin/av1top

print_info "Starting av1d service..."
systemctl start av1d

print_info "Checking service status..."
systemctl status av1d --no-pager

echo ""
print_info "Update complete!"
print_info "View logs with: journalctl -u av1d -f"
print_info "Monitor with: av1top"
