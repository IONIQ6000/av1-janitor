#!/bin/bash
set -e

# AV1 Janitor Complete Installation Script
# This script installs FFmpeg 8.0.1 from source with AV1 encoders and the av1-janitor application

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="/tmp/av1-janitor-build"
INSTALL_PREFIX="/usr/local"
FFMPEG_VERSION="8.0.1"
FFMPEG_URL="https://ffmpeg.org/releases/ffmpeg-${FFMPEG_VERSION}.tar.xz"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        DISTRO=$ID
        DISTRO_VERSION=$VERSION_ID
    else
        log_error "Cannot detect Linux distribution"
        exit 1
    fi
    log_info "Detected distribution: $DISTRO $DISTRO_VERSION"
}

install_build_dependencies() {
    log_info "Installing build dependencies..."
    
    case $DISTRO in
        ubuntu|debian)
            apt-get update
            apt-get install -y \
                build-essential \
                pkg-config \
                yasm \
                nasm \
                cmake \
                git \
                wget \
                curl \
                autoconf \
                automake \
                libtool \
                libx264-dev \
                libx265-dev \
                libvpx-dev \
                libopus-dev \
                libmp3lame-dev \
                libvorbis-dev \
                libass-dev \
                libfreetype6-dev \
                libfontconfig1-dev \
                libfribidi-dev \
                libharfbuzz-dev \
                libtheora-dev \
                libva-dev \
                libvdpau-dev \
                libxcb1-dev \
                libxcb-shm0-dev \
                libxcb-xfixes0-dev \
                texinfo \
                zlib1g-dev
            ;;
        fedora|rhel|centos)
            dnf install -y \
                gcc \
                gcc-c++ \
                make \
                pkg-config \
                yasm \
                nasm \
                cmake \
                git \
                wget \
                curl \
                autoconf \
                automake \
                libtool \
                x264-devel \
                x265-devel \
                libvpx-devel \
                opus-devel \
                lame-devel \
                libvorbis-devel \
                libass-devel \
                freetype-devel \
                fontconfig-devel \
                fribidi-devel \
                harfbuzz-devel \
                libtheora-devel \
                libva-devel \
                libvdpau-devel \
                libxcb-devel \
                texinfo \
                zlib-devel
            ;;
        arch|manjaro)
            pacman -Syu --noconfirm \
                base-devel \
                pkg-config \
                yasm \
                nasm \
                cmake \
                git \
                wget \
                curl \
                autoconf \
                automake \
                libtool \
                x264 \
                x265 \
                libvpx \
                opus \
                lame \
                libvorbis \
                libass \
                freetype2 \
                fontconfig \
                fribidi \
                harfbuzz \
                libtheora \
                libva \
                libvdpau \
                libxcb \
                texinfo \
                zlib
            ;;
        *)
            log_error "Unsupported distribution: $DISTRO"
            log_warn "Please install build dependencies manually"
            exit 1
            ;;
    esac
    
    log_info "Build dependencies installed successfully"
}

build_svt_av1() {
    log_info "Building SVT-AV1 encoder..."
    
    cd "$BUILD_DIR"
    
    if [ -d "SVT-AV1" ]; then
        rm -rf SVT-AV1
    fi
    
    git clone --depth 1 https://gitlab.com/AOMediaCodec/SVT-AV1.git
    cd SVT-AV1/Build
    cmake .. -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$INSTALL_PREFIX"
    make -j$(nproc)
    make install
    
    log_info "SVT-AV1 installed successfully"
}

build_aom() {
    log_info "Building libaom-av1 encoder..."
    
    cd "$BUILD_DIR"
    
    if [ -d "aom" ]; then
        rm -rf aom
    fi
    
    git clone --depth 1 https://aomedia.googlesource.com/aom
    cd aom
    mkdir -p build
    cd build
    cmake .. -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$INSTALL_PREFIX" -DENABLE_TESTS=0
    make -j$(nproc)
    make install
    
    log_info "libaom-av1 installed successfully"
}

build_rav1e() {
    log_info "Building rav1e encoder..."
    
    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        log_info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    cd "$BUILD_DIR"
    
    if [ -d "rav1e" ]; then
        rm -rf rav1e
    fi
    
    git clone --depth 1 https://github.com/xiph/rav1e.git
    cd rav1e
    cargo build --release
    cargo cinstall --release --prefix="$INSTALL_PREFIX"
    
    log_info "rav1e installed successfully"
}

build_ffmpeg() {
    log_info "Downloading FFmpeg ${FFMPEG_VERSION}..."
    
    cd "$BUILD_DIR"
    
    if [ -f "ffmpeg-${FFMPEG_VERSION}.tar.xz" ]; then
        rm -f "ffmpeg-${FFMPEG_VERSION}.tar.xz"
    fi
    
    wget "$FFMPEG_URL"
    tar -xf "ffmpeg-${FFMPEG_VERSION}.tar.xz"
    cd "ffmpeg-${FFMPEG_VERSION}"
    
    log_info "Configuring FFmpeg with AV1 encoders..."
    
    PKG_CONFIG_PATH="$INSTALL_PREFIX/lib/pkgconfig:$PKG_CONFIG_PATH" \
    ./configure \
        --prefix="$INSTALL_PREFIX" \
        --enable-gpl \
        --enable-version3 \
        --enable-nonfree \
        --enable-libsvtav1 \
        --enable-libaom \
        --enable-librav1e \
        --enable-libx264 \
        --enable-libx265 \
        --enable-libvpx \
        --enable-libopus \
        --enable-libmp3lame \
        --enable-libvorbis \
        --enable-libass \
        --enable-libfreetype \
        --enable-libfontconfig \
        --enable-libfribidi \
        --enable-libtheora \
        --enable-vaapi \
        --enable-vdpau \
        --enable-shared \
        --disable-static
    
    log_info "Building FFmpeg (this may take a while)..."
    make -j$(nproc)
    
    log_info "Installing FFmpeg..."
    make install
    
    # Update library cache
    ldconfig
    
    log_info "FFmpeg ${FFMPEG_VERSION} installed successfully"
}

verify_ffmpeg() {
    log_info "Verifying FFmpeg installation..."
    
    if ! command -v ffmpeg &> /dev/null; then
        log_error "FFmpeg not found in PATH"
        exit 1
    fi
    
    FFMPEG_VERSION_OUTPUT=$(ffmpeg -version | head -n 1)
    log_info "FFmpeg version: $FFMPEG_VERSION_OUTPUT"
    
    # Check for AV1 encoders
    log_info "Checking for AV1 encoders..."
    
    if ffmpeg -hide_banner -encoders 2>/dev/null | grep -q libsvtav1; then
        log_info "✓ SVT-AV1 encoder available"
    else
        log_warn "✗ SVT-AV1 encoder not available"
    fi
    
    if ffmpeg -hide_banner -encoders 2>/dev/null | grep -q libaom-av1; then
        log_info "✓ libaom-av1 encoder available"
    else
        log_warn "✗ libaom-av1 encoder not available"
    fi
    
    if ffmpeg -hide_banner -encoders 2>/dev/null | grep -q librav1e; then
        log_info "✓ librav1e encoder available"
    else
        log_warn "✗ librav1e encoder not available"
    fi
}

install_rust() {
    log_info "Checking Rust installation..."
    
    # Check if running as root
    if [[ $EUID -eq 0 ]]; then
        # Install for the user who invoked sudo
        REAL_USER=${SUDO_USER:-$USER}
        REAL_HOME=$(eval echo ~$REAL_USER)
        
        if ! sudo -u "$REAL_USER" bash -c 'command -v cargo &> /dev/null'; then
            log_info "Installing Rust for user $REAL_USER..."
            sudo -u "$REAL_USER" bash -c 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'
        else
            log_info "Rust already installed for user $REAL_USER"
        fi
    else
        if ! command -v cargo &> /dev/null; then
            log_info "Installing Rust..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
        else
            log_info "Rust already installed"
        fi
    fi
}

build_av1_janitor() {
    log_info "Building av1-janitor..."
    
    cd "$SCRIPT_DIR"
    
    # Build as the user who invoked sudo if running as root
    if [[ $EUID -eq 0 ]]; then
        REAL_USER=${SUDO_USER:-$USER}
        sudo -u "$REAL_USER" bash -c "source ~/.cargo/env && cargo build --release"
    else
        cargo build --release
    fi
    
    log_info "av1-janitor built successfully"
}

install_av1_janitor() {
    log_info "Installing av1-janitor binaries..."
    
    # Install binaries
    install -m 755 "$SCRIPT_DIR/target/release/av1d" "$INSTALL_PREFIX/bin/"
    install -m 755 "$SCRIPT_DIR/target/release/av1top" "$INSTALL_PREFIX/bin/"
    
    # Create directories
    mkdir -p /etc/av1d
    mkdir -p /var/lib/av1d/jobs
    mkdir -p /var/log/av1d
    
    # Install default configuration if it doesn't exist
    if [ ! -f /etc/av1d/config.toml ]; then
        cat > /etc/av1d/config.toml << 'EOF'
# AV1 Daemon Configuration

# Directories to scan for video files
library_roots = ["/path/to/your/media"]

# Minimum file size in bytes (100 MB default)
min_bytes = 104857600

# Maximum output size ratio (0.95 = output must be < 95% of original)
max_size_ratio = 0.95

# Scan interval in seconds
scan_interval_secs = 3600

# Job state directory
job_state_dir = "/var/lib/av1d/jobs"

# Temporary output directory
temp_output_dir = "/tmp/av1d"

# Maximum concurrent encoding jobs
max_concurrent_jobs = 1

# Preferred encoder: "svt", "aom", or "rav1e"
prefer_encoder = "svt"

# Quality tier: "high" or "very_high"
quality_tier = "high"

# Keep original files after successful encoding
keep_original = false

# Write .why.txt files explaining skip reasons
write_why_sidecars = true
EOF
        log_info "Default configuration created at /etc/av1d/config.toml"
        log_warn "Please edit /etc/av1d/config.toml to set your library_roots"
    fi
    
    # Install systemd service
    if [ -f "$SCRIPT_DIR/av1d.service" ]; then
        install -m 644 "$SCRIPT_DIR/av1d.service" /etc/systemd/system/
        systemctl daemon-reload
        log_info "Systemd service installed"
        log_info "Enable with: systemctl enable av1d"
        log_info "Start with: systemctl start av1d"
    fi
    
    log_info "av1-janitor installed successfully"
}

cleanup() {
    log_info "Cleaning up build directory..."
    rm -rf "$BUILD_DIR"
}

print_summary() {
    echo ""
    echo "=========================================="
    echo "  Installation Complete!"
    echo "=========================================="
    echo ""
    echo "Installed components:"
    echo "  - FFmpeg ${FFMPEG_VERSION} with AV1 encoders"
    echo "  - av1d (daemon): $INSTALL_PREFIX/bin/av1d"
    echo "  - av1top (TUI): $INSTALL_PREFIX/bin/av1top"
    echo ""
    echo "Configuration:"
    echo "  - Config file: /etc/av1d/config.toml"
    echo "  - Job state: /var/lib/av1d/jobs"
    echo "  - Logs: /var/log/av1d"
    echo ""
    echo "Next steps:"
    echo "  1. Edit /etc/av1d/config.toml to configure your media libraries"
    echo "  2. Enable the daemon: sudo systemctl enable av1d"
    echo "  3. Start the daemon: sudo systemctl start av1d"
    echo "  4. Monitor with TUI: av1top"
    echo ""
    echo "For more information, see README.md"
    echo ""
}

main() {
    log_info "Starting AV1 Janitor installation..."
    
    check_root
    detect_distro
    
    # Create build directory
    mkdir -p "$BUILD_DIR"
    
    # Install dependencies
    install_build_dependencies
    
    # Build AV1 encoders
    build_svt_av1
    build_aom
    build_rav1e
    
    # Build FFmpeg
    build_ffmpeg
    
    # Verify FFmpeg installation
    verify_ffmpeg
    
    # Install Rust (for building av1-janitor)
    install_rust
    
    # Build and install av1-janitor
    build_av1_janitor
    install_av1_janitor
    
    # Cleanup
    cleanup
    
    # Print summary
    print_summary
}

# Run main function
main
