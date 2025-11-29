# AV1 Re-encoding Daemon

A Rust-based AV1 video re-encoding daemon with a terminal user interface (TUI) for quality-first software AV1 encoding. Designed for 32-core AMD EPYC processors running in Debian containers, this system automatically discovers, classifies, and re-encodes video files to AV1 with intelligent quality optimization.

## Features

- **Automatic Discovery**: Recursively scans media libraries for video files
- **Intelligent Classification**: Detects WebRip vs Disc sources for optimal encoding
- **Quality-First Encoding**: Conservative CRF values and slower presets for maximum quality
- **Multi-Encoder Support**: SVT-AV1 (preferred), libaom-av1, and librav1e fallback
- **Safety Mechanisms**: Stable file detection, size gates, atomic file replacement
- **Real-Time Monitoring**: Terminal UI with progress tracking and system metrics
- **Concurrent Processing**: Configurable parallel encoding with resource management
- **State Persistence**: JSON-based job tracking for monitoring and recovery

## Quick Start

### Installation

#### Option 1: Docker (Recommended)

```bash
# Build and run with Docker Compose
docker-compose up -d

# Monitor with TUI
docker exec -it av1d av1top
```

#### Option 2: Native Debian Installation

```bash
# Build the project
cargo build --release

# Run installation script
sudo ./install.sh

# Start the service
sudo systemctl start av1d

# Monitor with TUI
av1top
```

See [DEPLOYMENT.md](DEPLOYMENT.md) for detailed installation instructions.

### Basic Configuration

Edit `/etc/av1d/config.toml` (or `config.toml` for Docker):

```toml
# Directories to scan
library_roots = ["/media/movies", "/media/tv"]

# Minimum file size (2 GiB)
min_bytes = 2147483648

# Quality settings
prefer_encoder = "svt"
quality_tier = "very_high"

# Concurrency (start with 1 for quality)
max_concurrent_jobs = 1

# Size gate (reject if output >= 90% of original)
max_size_ratio = 0.90
```

## Usage

### Running the Daemon

**Native Installation:**
```bash
# Start the service
sudo systemctl start av1d

# Enable auto-start on boot
sudo systemctl enable av1d

# Check status
sudo systemctl status av1d

# View logs
sudo journalctl -u av1d -f
```

**Docker:**
```bash
# Start container
docker-compose up -d

# View logs
docker logs -f av1d

# Stop container
docker-compose down
```

### Monitoring with the TUI

**Native Installation:**
```bash
av1top
```

**Docker:**
```bash
docker exec -it av1d av1top
```

**TUI Controls:**
- `↑/k`: Move selection up
- `↓/j`: Move selection down
- `Enter`: View job details
- `f`: Cycle filters (All, Pending, Running, Success, Failed)
- `s`: Cycle sort modes (Date, Size, Status, Savings)
- `q`: Quit

### Skipping Files

To prevent a file from being encoded, create a `.av1skip` marker:

```bash
# Skip a specific file
touch "/path/to/video.mkv.av1skip"

# Skip all files in a directory
find /path/to/directory -name "*.mkv" -exec touch "{}.av1skip" \;
```

The daemon will skip files with this marker and log the reason if `write_why_sidecars = true`.

### Checking Job Status

Job state is stored as JSON files in `/var/lib/av1d/jobs/`:

```bash
# List all jobs
ls -lh /var/lib/av1d/jobs/

# View a specific job
cat /var/lib/av1d/jobs/<job-id>.json | jq

# Count jobs by status
jq -r .status /var/lib/av1d/jobs/*.json | sort | uniq -c
```

## Configuration Options

### Library Scanning

- `library_roots`: Array of directories to scan recursively
- `min_bytes`: Minimum file size to consider (default: 2 GiB)
- `scan_interval_secs`: Time between scans (default: 60 seconds)

### Encoding Quality

- `prefer_encoder`: Preferred encoder - `"svt"`, `"aom"`, or `"rav1e"` (default: `"svt"`)
- `quality_tier`: Quality level - `"high"` or `"very_high"` (default: `"very_high"`)

**Automatic CRF Selection:**
- 2160p (4K) and above: CRF 21
- 1440p (2K): CRF 22
- 1080p (FHD): CRF 23
- Below 1080p: CRF 24
- Low bitrate sources: CRF +1

**Automatic Preset Selection (SVT-AV1):**
- 2160p and above: Preset 3
- 1440p: Preset 4
- 1080p: Preset 4
- Below 1080p: Preset 5
- Very high quality tier: Preset -1

### Output Validation

- `max_size_ratio`: Maximum output size as ratio of original (default: 0.90)
  - Encodes producing files ≥ 90% of original size are rejected

### Concurrency

- `max_concurrent_jobs`: Maximum parallel encoding jobs (default: 1)
  - Start with 1 for maximum quality
  - Increase to 2-3 if CPU utilization is low (<70%)
  - Each 4K encode uses 2-4 GB RAM

### File Management

- `job_state_dir`: Directory for job JSON files (default: `/var/lib/av1d/jobs`)
- `temp_output_dir`: Directory for temporary files (default: `/var/lib/av1d/temp`)
- `keep_original`: Keep original files as `.orig` (default: `false`)
- `write_why_sidecars`: Write `.why.txt` files for skipped files (default: `true`)

See `config.toml` for complete documentation of all options.

## How It Works

### Processing Pipeline

1. **Scan**: Recursively discover video files in `library_roots`
2. **Stability Check**: Wait 10 seconds to ensure file isn't being written
3. **Skip Marker Check**: Skip files with `.av1skip` markers
4. **Probe**: Extract metadata using ffprobe
5. **Gates**: Evaluate skip conditions (size, codec, no video streams)
6. **Classify**: Determine source type (WebLike vs DiscLike)
7. **Encode**: Build and execute FFmpeg command with optimal parameters
8. **Validate**: Verify output has exactly one AV1 stream and correct duration
9. **Size Gate**: Reject if output doesn't achieve sufficient compression
10. **Replace**: Atomically replace original with encoded output

### Source Classification

The daemon classifies sources to apply appropriate encoding safeguards:

**WebLike Sources** (streaming rips):
- Path contains: WEB, WEBRip, WEBDL, NF, AMZN, DSNP, HULU, ATVP
- Low bitrate for resolution
- Applies special FFmpeg flags: `-fflags +genpts -copyts -start_at_zero`
- Applies pad filter for odd dimensions

**DiscLike Sources** (physical media):
- Path contains: BluRay, Remux, BDMV, UHD
- High bitrate for resolution
- Standard encoding without special flags

### Automatic Skipping

Files are automatically skipped if:
- Already encoded in AV1 codec
- Smaller than `min_bytes` threshold
- Have `.av1skip` marker file
- No video streams detected
- Currently being written (unstable)

Skip reasons are written to `.why.txt` files when `write_why_sidecars = true`.

## Troubleshooting

### Daemon Won't Start

**Check logs:**
```bash
# Native
sudo journalctl -u av1d -n 50

# Docker
docker logs av1d
```

**Common issues:**
- FFmpeg not found or version < 8.0
  - Solution: Install FFmpeg 8.0+ with AV1 encoder support
- No AV1 encoders available
  - Solution: Ensure FFmpeg has libsvtav1, libaom-av1, or librav1e
- Configuration file invalid
  - Solution: Validate TOML syntax in `config.toml`
- Permissions on `/var/lib/av1d`
  - Solution: `sudo chown -R av1d:av1d /var/lib/av1d`

### No Files Being Processed

**Check:**
1. `library_roots` points to correct directories
2. Files are larger than `min_bytes` (default: 2 GiB)
3. Files don't have `.av1skip` markers
4. Files aren't already AV1 encoded
5. Check `.why.txt` files for skip reasons

**Verify file discovery:**
```bash
# Check if daemon can see files
find /media -type f \( -name "*.mkv" -o -name "*.mp4" \) -size +2G
```

### Encoding Failures

**View job details:**
- Use TUI: Press Enter on failed job
- Check JSON: `cat /var/lib/av1d/jobs/<job-id>.json | jq .reason`

**Common issues:**
- Corrupted source file
  - Solution: Verify source with `ffprobe -v error <file>`
- Insufficient disk space
  - Solution: Ensure 2x largest file space in `temp_output_dir`
- FFmpeg command failure
  - Solution: Check logs for FFmpeg error output

### Size Gate Rejections

If many encodes are rejected by the size gate:

**Possible causes:**
- Source already well-compressed (HEVC, VP9)
- Source already AV1 (shouldn't happen, but check)
- CRF too conservative for source quality

**Solutions:**
- Lower `max_size_ratio` (e.g., 0.85 or 0.80)
- Review `.why.txt` files for patterns
- Check source codecs: `ffprobe -v error -select_streams v:0 -show_entries stream=codec_name -of default=noprint_wrappers=1:nokey=1 <file>`

### High Memory Usage

**Each 4K encode uses 2-4 GB RAM**

**Solutions:**
- Reduce `max_concurrent_jobs`
- Monitor with TUI system metrics
- Check available memory: `free -h`

### Slow Encoding Speed

**Check:**
1. CPU utilization in TUI (should be high for quality encoding)
2. Disk I/O (place `temp_output_dir` on fast NVMe)
3. Preset values (slower presets = higher quality but slower)

**Optimization:**
- Ensure `temp_output_dir` is on fast storage
- Increase `max_concurrent_jobs` if CPU < 70% utilized
- Consider `quality_tier = "high"` instead of `"very_high"` if you need faster encodes

### TUI Not Showing Jobs

**Check:**
- `job_state_dir` path is correct
- Permissions allow reading JSON files
- Jobs exist: `ls /var/lib/av1d/jobs/`

**Verify:**
```bash
# Native
av1top --job-state-dir /var/lib/av1d/jobs

# Docker
docker exec -it av1d av1top
```

## Project Structure

This workspace contains three crates:

### `crates/daemon` - Core Library (`av1d-daemon`)
The core daemon library containing all business logic:
- **config**: Configuration loading and validation
- **startup**: FFmpeg version checking and encoder detection
- **scan**: Recursive directory scanning for video files
- **stable**: Stable file detection (prevents encoding files being written)
- **probe**: FFprobe metadata extraction
- **classify**: Source classification (WebLike vs DiscLike)
- **gates**: Pre-encoding gate evaluation (size, codec, skip markers)
- **encode**: FFmpeg command construction and execution
  - `svt`: SVT-AV1 encoder
  - `aom`: libaom-av1 encoder
  - `rav1e`: librav1e encoder
  - `common`: Shared command components
- **validate**: Output validation
- **size_gate**: Post-encoding size gate enforcement
- **replace**: Atomic file replacement
- **sidecars**: `.av1skip` and `.why.txt` file management
- **jobs**: Job lifecycle and state persistence

### `crates/cli-daemon` - Daemon Binary (`av1d`)
The daemon executable that runs the background encoding service.

### `crates/cli-tui` - TUI Binary (`av1top`)
The terminal user interface for monitoring encoding jobs in real-time.

## Building from Source

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- FFmpeg 8.0+ with AV1 encoder support
- Debian Bookworm or compatible Linux distribution

### Build Commands

```bash
# Build all crates
cargo build --workspace

# Build release binaries (optimized)
cargo build --release --workspace

# Build specific binary
cargo build --bin av1d
cargo build --bin av1top

# Run tests
cargo test --workspace

# Run tests for specific crate
cargo test -p av1d-daemon
```

### Installing FFmpeg with AV1 Support

**Debian/Ubuntu:**
```bash
# Add deb-multimedia repository for latest FFmpeg
sudo apt update
sudo apt install ffmpeg

# Verify version and encoders
ffmpeg -version
ffmpeg -hide_banner -encoders | grep av1
```

**From Source:**
```bash
# Build FFmpeg with SVT-AV1 support
# See: https://trac.ffmpeg.org/wiki/CompilationGuide
```

## Dependencies

- **tokio**: Async runtime for concurrent operations
- **serde/serde_json**: Serialization for configuration and job state
- **toml**: Configuration file parsing
- **anyhow/thiserror**: Error handling and propagation
- **chrono**: Date/time handling for timestamps
- **uuid**: Unique job ID generation
- **tracing**: Structured logging
- **walkdir**: Recursive directory traversal
- **regex**: Pattern matching for classification
- **ratatui**: Terminal UI framework
- **crossterm**: Terminal manipulation and input
- **sysinfo**: System metrics (CPU, memory, GPU)
- **proptest**: Property-based testing framework

## Performance Tuning

See [DEPLOYMENT.md](DEPLOYMENT.md) for detailed performance tuning guidance, including:
- EPYC-specific recommendations
- Quality vs speed tradeoffs
- Storage considerations
- Memory optimization
- Concurrent job tuning

## Documentation

- **[DEPLOYMENT.md](DEPLOYMENT.md)**: Complete deployment guide with container and native installation
- **[PACKAGING.md](PACKAGING.md)**: Packaging files overview and quick start
- **config.toml**: Comprehensive configuration reference with inline documentation

## Requirements

- Rust 1.70+
- FFmpeg 8.0+ with AV1 encoder support (libsvtav1, libaom-av1, or librav1e)
- Debian Bookworm or compatible Linux distribution (for native installation)
- Docker (for container deployment)

## License

See LICENSE file for details.
