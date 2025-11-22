# Deployment Guide

This guide covers deploying the AV1 Re-encoding Daemon in a Debian container environment.

## Prerequisites

- Docker (for container deployment)
- Debian Bookworm or compatible Linux distribution (for native deployment)
- FFmpeg >= 8.0
- Rust toolchain (for building from source)

## Building from Source

```bash
# Build release binaries
cargo build --release

# Binaries will be available at:
# - target/release/av1d (daemon)
# - target/release/av1top (TUI monitor)
```

## Container Deployment

### Building the Docker Image

```bash
# Build the container image
docker build -t av1-reencoder:latest .

# Verify the image
docker images | grep av1-reencoder
```

### Running the Container

```bash
# Run with volume mounts for media library and persistent state
docker run -d \
  --name av1d \
  --restart unless-stopped \
  -v /path/to/media:/media \
  -v /path/to/config:/etc/av1d \
  -v /path/to/state:/var/lib/av1d \
  av1-reencoder:latest

# View logs
docker logs -f av1d

# Monitor with TUI (in separate terminal)
docker exec -it av1d av1top
```

### Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  av1d:
    image: av1-reencoder:latest
    container_name: av1d
    restart: unless-stopped
    volumes:
      - /path/to/media:/media
      - ./config.toml:/etc/av1d/config.toml:ro
      - av1d-state:/var/lib/av1d
    environment:
      - RUST_LOG=info

volumes:
  av1d-state:
```

Run with:
```bash
docker-compose up -d
```

## Native Debian Installation

### Using the Installation Script

```bash
# 1. Build the project
cargo build --release

# 2. Run the installation script as root
sudo ./install.sh

# 3. Edit configuration if needed
sudo nano /etc/av1d/config.toml

# 4. Enable and start the service
sudo systemctl enable av1d
sudo systemctl start av1d

# 5. Check status
sudo systemctl status av1d
```

### Manual Installation

If you prefer manual installation:

```bash
# Create user
sudo useradd -r -s /bin/false av1d

# Create directories
sudo mkdir -p /etc/av1d /var/lib/av1d/jobs /var/lib/av1d/temp

# Copy binaries
sudo cp target/release/av1d /usr/local/bin/
sudo cp target/release/av1top /usr/local/bin/
sudo chmod +x /usr/local/bin/av1d /usr/local/bin/av1top

# Copy configuration
sudo cp config.toml /etc/av1d/config.toml
sudo chown root:av1d /etc/av1d/config.toml
sudo chmod 640 /etc/av1d/config.toml

# Set permissions
sudo chown -R av1d:av1d /var/lib/av1d

# Install systemd service
sudo cp av1d.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable av1d
sudo systemctl start av1d
```

## Configuration

Edit `/etc/av1d/config.toml` to customize behavior:

```toml
# Directories to scan
library_roots = ["/media/movies", "/media/tv"]

# Minimum file size (2 GiB)
min_bytes = 2147483648

# Quality settings
prefer_encoder = "svt"
quality_tier = "high"

# Concurrency (start with 1 for quality)
max_concurrent_jobs = 1

# Size gate (reject if output >= 90% of original)
max_size_ratio = 0.90
```

See `config.toml` for full documentation of all options.

## Systemd Service Management

### Service Control

```bash
# Start the daemon
sudo systemctl start av1d

# Stop the daemon
sudo systemctl stop av1d

# Restart the daemon
sudo systemctl restart av1d

# Enable auto-start on boot
sudo systemctl enable av1d

# Disable auto-start on boot
sudo systemctl disable av1d

# Check service status
sudo systemctl status av1d

# View service configuration
systemctl cat av1d
```

### Service Status Information

The `systemctl status av1d` command shows:
- Active/inactive state
- Process ID (PID)
- Memory usage
- Recent log entries
- Uptime and restart count

Example output:
```
● av1d.service - AV1 Re-encoding Daemon
     Loaded: loaded (/etc/systemd/system/av1d.service; enabled; vendor preset: enabled)
     Active: active (running) since Sat 2024-01-20 10:30:00 UTC; 2h 15min ago
   Main PID: 12345 (av1d)
      Tasks: 8 (limit: 4915)
     Memory: 512.0M
        CPU: 45min 30s
     CGroup: /system.slice/av1d.service
             └─12345 /usr/local/bin/av1d --config /etc/av1d/config.toml
```

### Reloading Configuration

After editing `/etc/av1d/config.toml`:

```bash
# Restart the service to apply changes
sudo systemctl restart av1d

# Verify the service started successfully
sudo systemctl status av1d
```

Note: Configuration changes require a service restart. The daemon does not hot-reload configuration.

### Service Logs

```bash
# View recent logs
sudo journalctl -u av1d

# Follow logs in real-time
sudo journalctl -u av1d -f

# View logs since boot
sudo journalctl -u av1d -b

# View logs from last hour
sudo journalctl -u av1d --since "1 hour ago"

# View logs with specific priority (error and above)
sudo journalctl -u av1d -p err

# Export logs to file
sudo journalctl -u av1d > av1d-logs.txt
```

### Automatic Restart

The service is configured to automatically restart on failure:
- `Restart=always`: Restart on any exit (clean or failure)
- `RestartSec=10`: Wait 10 seconds before restarting

To modify restart behavior, edit `/etc/systemd/system/av1d.service`:
```ini
[Service]
Restart=on-failure  # Only restart on failure
RestartSec=30       # Wait 30 seconds before restart
```

Then reload systemd:
```bash
sudo systemctl daemon-reload
sudo systemctl restart av1d
```

## Monitoring

### Using the TUI

```bash
# Native installation
av1top

# Docker container
docker exec -it av1d av1top
```

The TUI displays:
- Active and completed jobs
- Encoding progress and ETA
- System resource usage (CPU, memory, GPU)
- Aggregate statistics (space saved, success rate)

**TUI Controls:**
- `↑/k`: Move selection up
- `↓/j`: Move selection down
- `Enter`: View detailed job information
- `f`: Cycle through filters (All, Pending, Running, Success, Failed)
- `s`: Cycle through sort modes (Date, Size, Status, Savings)
- `q`: Quit

**TUI Sections:**

1. **Header**: System metrics and aggregate statistics
   - CPU usage percentage
   - Memory usage (used/total)
   - GPU usage (if available)
   - Total jobs, success rate, space saved

2. **Job Table**: List of encoding jobs
   - Status (Pending, Running, Success, Failed, Skipped)
   - Filename
   - Original and new file sizes
   - Compression ratio
   - Progress bar (for running jobs)
   - ETA (for running jobs)

3. **Detail View** (press Enter on selected job):
   - Full file path
   - Video resolution and codec
   - Bitrate and frame rate
   - Encoding parameters (CRF, preset, encoder)
   - Timestamps (created, started, finished)
   - Failure reason (if failed)
   - Size savings and compression ratio

### Viewing Logs

```bash
# Native installation
sudo journalctl -u av1d -f

# Docker container
docker logs -f av1d
```

**Log Levels:**
- `ERROR`: Critical failures requiring attention
- `WARN`: Non-critical issues (skipped files, validation failures)
- `INFO`: Normal operations (job started, completed, skipped)
- `DEBUG`: Detailed information (FFmpeg commands, file operations)

**Setting Log Level:**

Edit `/etc/systemd/system/av1d.service` (native) or `docker-compose.yml` (Docker):
```bash
# Native: Add to [Service] section
Environment="RUST_LOG=debug"

# Docker: Add to environment section
environment:
  - RUST_LOG=debug
```

### Job State Files

Job metadata is stored as JSON files in `/var/lib/av1d/jobs/`:

```bash
# List all jobs
ls -lh /var/lib/av1d/jobs/

# View a specific job
cat /var/lib/av1d/jobs/<job-id>.json | jq

# Count jobs by status
jq -r .status /var/lib/av1d/jobs/*.json | sort | uniq -c

# Find failed jobs
jq -r 'select(.status == "Failed") | .source_path' /var/lib/av1d/jobs/*.json

# Calculate total space saved
jq -r 'select(.status == "Success") | (.original_bytes - .new_bytes)' /var/lib/av1d/jobs/*.json | awk '{sum+=$1} END {print sum/1024/1024/1024 " GB"}'

# Find jobs with specific encoder
jq -r 'select(.encoder_used == "libsvtav1") | .source_path' /var/lib/av1d/jobs/*.json
```

### Monitoring System Resources

**CPU Usage:**
```bash
# Overall CPU usage
top -bn1 | grep "Cpu(s)"

# Per-process CPU usage
ps aux | grep av1d

# Real-time monitoring
htop -p $(pgrep av1d)
```

**Memory Usage:**
```bash
# Overall memory
free -h

# Daemon memory usage
ps aux | grep av1d | awk '{print $6/1024 " MB"}'

# Detailed memory breakdown
sudo pmap $(pgrep av1d)
```

**Disk Usage:**
```bash
# Check temp directory space
df -h /var/lib/av1d/temp

# Check job state directory
du -sh /var/lib/av1d/jobs

# Monitor disk I/O
iostat -x 1
```

**FFmpeg Processes:**
```bash
# List running FFmpeg processes
ps aux | grep ffmpeg

# Count concurrent encodes
pgrep -c ffmpeg

# Monitor FFmpeg resource usage
top -p $(pgrep -d',' ffmpeg)
```

### Health Checks

**Verify Daemon is Running:**
```bash
# Check process
pgrep -a av1d

# Check systemd status
systemctl is-active av1d

# Check listening ports (if applicable)
sudo netstat -tlnp | grep av1d
```

**Verify FFmpeg Availability:**
```bash
# Check FFmpeg version
ffmpeg -version | head -n1

# Check AV1 encoders
ffmpeg -hide_banner -encoders | grep av1
```

**Verify File Permissions:**
```bash
# Check job state directory
ls -ld /var/lib/av1d/jobs
ls -l /var/lib/av1d/jobs/ | head

# Check temp directory
ls -ld /var/lib/av1d/temp

# Check configuration
ls -l /etc/av1d/config.toml
```

### Alerting and Notifications

**Email Notifications on Failure:**

Create a systemd override to send email on failure:
```bash
sudo systemctl edit av1d
```

Add:
```ini
[Unit]
OnFailure=failure-notification@%n.service
```

Create notification service:
```bash
sudo nano /etc/systemd/system/failure-notification@.service
```

```ini
[Unit]
Description=Send notification on %i failure

[Service]
Type=oneshot
ExecStart=/usr/local/bin/send-notification.sh %i
```

**Monitoring with External Tools:**

- **Prometheus**: Export metrics via custom exporter
- **Grafana**: Visualize job statistics and system metrics
- **Nagios/Icinga**: Monitor service status and resource usage
- **Zabbix**: Track encoding throughput and success rate

## Performance Tuning

### For 32-core EPYC Processor

1. **Start Conservative**: Begin with `max_concurrent_jobs = 1`
   - SVT-AV1 preset 3-4 will utilize most cores for 4K content
   - Monitor CPU usage with the TUI

2. **Increase Gradually**: If CPU utilization is low (<70%)
   - Increase to `max_concurrent_jobs = 2`
   - Monitor quality and encoding speed
   - Adjust based on your quality requirements

3. **Memory Considerations**:
   - Each 4K encode uses 2-4 GB RAM
   - Ensure sufficient memory for concurrent jobs
   - Monitor with TUI system metrics

4. **Storage Optimization**:
   - Place `temp_output_dir` on fast NVMe storage
   - Ensure 2x largest video file space available
   - Monitor disk I/O if encoding seems slow

### Quality vs Speed Tradeoffs

```toml
# Maximum quality (slowest)
quality_tier = "very_high"
max_concurrent_jobs = 1

# High quality (recommended)
quality_tier = "high"
max_concurrent_jobs = 1

# Balanced (faster, slight quality loss)
quality_tier = "high"
max_concurrent_jobs = 2
```

## Troubleshooting

### Daemon Won't Start

```bash
# Check logs
sudo journalctl -u av1d -n 50

# Common issues:
# - FFmpeg not found or version < 8.0
# - No AV1 encoders available
# - Configuration file invalid
# - Permissions on /var/lib/av1d
```

### No Files Being Processed

Check:
1. `library_roots` points to correct directories
2. Files are larger than `min_bytes`
3. Files don't have `.av1skip` markers
4. Files aren't already AV1 encoded
5. Check `.why.txt` files for skip reasons

### Encoding Failures

```bash
# View job details in TUI (press Enter on failed job)
# Check job JSON for error details
cat /var/lib/av1d/jobs/<job-id>.json | jq .reason

# Common issues:
# - Corrupted source file
# - Insufficient disk space
# - FFmpeg command failure
```

### Size Gate Rejections

If many encodes are rejected by the size gate:
- Source may already be well-compressed
- Consider lowering `max_size_ratio` (e.g., 0.85)
- Check if source is already AV1 or HEVC
- Review `.why.txt` files for details

## Uninstallation

### Native Installation

```bash
# Stop and disable service
sudo systemctl stop av1d
sudo systemctl disable av1d

# Remove files
sudo rm /usr/local/bin/av1d /usr/local/bin/av1top
sudo rm /etc/systemd/system/av1d.service
sudo rm -rf /etc/av1d
sudo rm -rf /var/lib/av1d

# Remove user
sudo userdel av1d

# Reload systemd
sudo systemctl daemon-reload
```

### Docker

```bash
# Stop and remove container
docker stop av1d
docker rm av1d

# Remove image
docker rmi av1-reencoder:latest

# Remove volumes (if desired)
docker volume rm av1d-state
```

## Security Considerations

1. **User Isolation**: Daemon runs as non-root user `av1d`
2. **File Permissions**: Configuration is readable only by root and av1d group
3. **Resource Limits**: Systemd service includes memory limits
4. **Read-Only Config**: Mount configuration as read-only in containers
5. **Network**: Daemon doesn't require network access

## Support

For issues, questions, or contributions:
- Check logs first: `journalctl -u av1d -f`
- Review job state files in `/var/lib/av1d/jobs/`
- Check `.why.txt` files alongside skipped videos
- Monitor with TUI for real-time insights
