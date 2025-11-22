# Packaging Files

This directory contains all necessary files for deploying the AV1 Re-encoding Daemon.

## Files Overview

### Core Packaging Files

- **`Dockerfile`** - Multi-stage Docker build configuration
  - Stage 1: Builds Rust binaries from source
  - Stage 2: Creates minimal Debian runtime image with ffmpeg
  - Uses debian:bookworm-slim base image
  - Runs as non-root user `av1d`

- **`docker-compose.yml`** - Docker Compose configuration
  - Simplifies container deployment
  - Includes volume mounts for media and state
  - Configurable resource limits
  - Example configuration ready to customize

- **`.dockerignore`** - Docker build optimization
  - Excludes unnecessary files from build context
  - Reduces image size and build time

### System Integration Files

- **`av1d.service`** - Systemd service unit file
  - Manages daemon lifecycle
  - Automatic restart on failure
  - Security hardening (NoNewPrivileges, ProtectSystem)
  - Resource limits for memory and file descriptors
  - Runs as non-root user `av1d`

- **`install.sh`** - Automated installation script
  - Creates system user and directories
  - Installs binaries and configuration
  - Sets up systemd service
  - Verifies ffmpeg installation
  - Provides post-installation instructions

### Configuration

- **`config.toml`** - Default daemon configuration
  - Comprehensive documentation of all options
  - Quality-first defaults for 32-core EPYC
  - Sensible values for production use
  - Includes usage notes and examples

### Documentation

- **`DEPLOYMENT.md`** - Complete deployment guide
  - Container deployment instructions
  - Native Debian installation steps
  - Configuration reference
  - Monitoring and troubleshooting
  - Performance tuning guidelines
  - Security considerations

## Quick Start

### Docker Deployment

```bash
# Build and run with Docker Compose
docker-compose up -d

# Monitor with TUI
docker exec -it av1d av1top
```

### Native Installation

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

## File Locations After Installation

### Native Installation
- Binaries: `/usr/local/bin/av1d`, `/usr/local/bin/av1top`
- Configuration: `/etc/av1d/config.toml`
- Job state: `/var/lib/av1d/jobs/`
- Temp files: `/var/lib/av1d/temp/`
- Service file: `/etc/systemd/system/av1d.service`
- Logs: `journalctl -u av1d`

### Container Deployment
- Binaries: `/usr/local/bin/av1d`, `/usr/local/bin/av1top`
- Configuration: `/etc/av1d/config.toml` (mounted from host)
- Job state: `/var/lib/av1d/jobs/` (persistent volume)
- Temp files: `/var/lib/av1d/temp/` (persistent volume)
- Media: `/media` (mounted from host)
- Logs: `docker logs av1d`

## Requirements Validation

This packaging implementation satisfies:

- **Requirement 30.1**: Uses debian:bookworm-slim base image ✓
- **Requirement 30.2**: Installs ffmpeg system dependency ✓
- **Requirement 30.3**: Includes systemd service file with restart policies ✓
- **Requirement 30.4**: Provides default configuration file with documentation ✓
- **Requirement 30.5**: Installation script creates directories and sets permissions ✓

## Security Features

1. **Non-root execution**: Daemon runs as dedicated `av1d` user
2. **Minimal permissions**: Configuration readable only by root and av1d group
3. **Systemd hardening**: NoNewPrivileges, ProtectSystem, ProtectHome
4. **Resource limits**: Memory and file descriptor limits
5. **Read-only config**: Configuration mounted read-only in containers

## Customization

### Adjusting for Your Environment

Edit `config.toml` before installation:
```toml
# Your media directories
library_roots = ["/mnt/movies", "/mnt/tv"]

# Adjust for your hardware
max_concurrent_jobs = 2  # Increase if CPU underutilized

# Tune quality vs speed
quality_tier = "very_high"  # Maximum quality
```

Edit `docker-compose.yml` for container deployment:
```yaml
volumes:
  - /your/media/path:/media
  - ./config.toml:/etc/av1d/config.toml:ro
```

## Support

For detailed instructions, see:
- **DEPLOYMENT.md** - Complete deployment guide
- **config.toml** - Configuration reference
- **README.md** - Project overview

For troubleshooting:
- Check logs: `journalctl -u av1d -f` or `docker logs -f av1d`
- Monitor with TUI: `av1top`
- Review job state: `/var/lib/av1d/jobs/*.json`
- Check skip reasons: `.why.txt` files alongside videos
