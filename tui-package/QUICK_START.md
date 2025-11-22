# Quick Start Guide

Get the TUI running in 5 minutes!

## Prerequisites

- Rust 1.70+ installed
- Terminal with 256 color support
- Minimum terminal size: 80x12

## Installation

### Option 1: Standalone Binary

```bash
cd tui-package
cargo build --release
./target/release/tui --job-state-dir /path/to/jobs --temp-output-dir /tmp
```

### Option 2: Add to Existing Project

```bash
# Copy source files
cp -r tui-package/src/models your-project/src/
cp tui-package/src/main.rs your-project/src/bin/tui.rs

# Add dependencies to Cargo.toml
[dependencies]
ratatui = "0.27"
crossterm = "0.28"
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
sysinfo = "0.31"
humansize = "2.1"
clap = { version = "4", features = ["derive"] }
uuid = { version = "1.10", features = ["v4", "serde"] }

# Build and run
cargo build --release
cargo run --bin tui -- --job-state-dir /path/to/jobs
```

## Usage

### Command Line Options

```bash
tui --job-state-dir <DIR> --temp-output-dir <DIR>
```

- `--job-state-dir`: Directory containing job JSON files (required)
- `--temp-output-dir`: Directory for temporary files (required)

### Keyboard Controls

| Key | Action |
|-----|--------|
| `q` | Quit |
| `↑` or `k` | Move selection up |
| `↓` or `j` | Move selection down |
| `PgUp` or `u` | Page up (10 items) |
| `PgDn` or `d` | Page down (10 items) |
| `f` | Cycle filter (All → Pending → Running → Success → Failed) |
| `s` | Cycle sort (Date → Size → Status → Savings) |
| `Enter` | Toggle detail view |
| `r` | Requeue running job |
| `R` | Force refresh |

## Job File Format

Jobs are stored as JSON files in the job state directory:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "source_path": "/media/video.mkv",
  "status": "running",
  "created_at": "2024-01-15T10:30:00Z",
  "started_at": "2024-01-15T10:31:00Z",
  "finished_at": null,
  "original_bytes": 5000000000,
  "new_bytes": null,
  "video_codec": "hevc",
  "video_width": 1920,
  "video_height": 1080,
  "video_bitrate": 8000000,
  "video_frame_rate": "24/1"
}
```

### Required Fields

- `id` (string) - Unique identifier
- `status` (string) - One of: "pending", "running", "success", "failed", "skipped"
- `created_at` (ISO 8601 timestamp)
- `source_path` (string) - Path to source file

### Optional Fields

- `started_at` (ISO 8601 timestamp)
- `finished_at` (ISO 8601 timestamp)
- `original_bytes` (number) - Original file size
- `new_bytes` (number) - New file size after processing
- `reason` (string) - Failure/skip reason
- Domain-specific fields (customize as needed)

## UI Layout

```
┌─────────────────────────────────────────────────────────────┐
│ Header: System Metrics (CPU, Memory, GPU)                   │
├─────────────────────────────────────────────────────────────┤
│ Statistics: Aggregate metrics and trends                    │
├─────────────────────────────────────────────────────────────┤
│ Current Job: Active job progress (if running)               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ Job Table: Scrollable list of all jobs                      │
│                                                              │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│ Status Bar: Filter, Sort, Help                              │
└─────────────────────────────────────────────────────────────┘
```

## Customization

### Change Colors

Edit `ColorScheme` in `src/main.rs`:

```rust
impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            pending: Color::Yellow,
            running: Color::Green,
            success: Color::Blue,
            failed: Color::Red,
            // ... customize other colors
        }
    }
}
```

### Add Custom Columns

1. Add to `TableColumn` enum
2. Implement `header()` and `width()` methods
3. Update rendering logic in `render_job_table()`

### Modify Refresh Rate

Change in `main()` function:

```rust
let tick_rate = Duration::from_millis(250); // Default
let tick_rate = Duration::from_millis(100); // Faster
let tick_rate = Duration::from_millis(1000); // Slower
```

## Troubleshooting

### "No jobs found"

- Check that job state directory exists
- Verify JSON files are valid
- Ensure files have `.json` extension

### Terminal rendering issues

```bash
# Set terminal type
export TERM=xterm-256color

# Check terminal size
tput cols  # Should be >= 80
tput lines # Should be >= 12
```

### High CPU usage

- Increase tick rate (slower refresh)
- Reduce number of jobs displayed
- Disable expensive statistics calculations

### GPU metrics not showing

GPU detection works for Intel Arc GPUs. For other GPUs:

1. Modify `get_gpu_usage()` in `src/main.rs`
2. Add your GPU-specific detection logic
3. Or return 0.0 to disable GPU metrics

## Examples

### Create test jobs

```bash
mkdir -p /tmp/test-jobs

cat > /tmp/test-jobs/job1.json << 'EOF'
{
  "id": "test-1",
  "source_path": "/media/video1.mkv",
  "status": "pending",
  "created_at": "2024-01-15T10:00:00Z",
  "original_bytes": 5000000000
}
EOF

cat > /tmp/test-jobs/job2.json << 'EOF'
{
  "id": "test-2",
  "source_path": "/media/video2.mkv",
  "status": "running",
  "created_at": "2024-01-15T10:05:00Z",
  "started_at": "2024-01-15T10:06:00Z",
  "original_bytes": 3000000000
}
EOF

# Run TUI
./target/release/tui --job-state-dir /tmp/test-jobs --temp-output-dir /tmp
```

### Watch jobs update in real-time

```bash
# Terminal 1: Run TUI
./target/release/tui --job-state-dir /tmp/jobs --temp-output-dir /tmp

# Terminal 2: Update job status
cat > /tmp/jobs/job1.json << 'EOF'
{
  "id": "job1",
  "status": "success",
  "created_at": "2024-01-15T10:00:00Z",
  "started_at": "2024-01-15T10:01:00Z",
  "finished_at": "2024-01-15T10:30:00Z",
  "source_path": "/media/video.mkv",
  "original_bytes": 5000000000,
  "new_bytes": 2500000000
}
EOF
```

## Next Steps

- Read [README.md](README.md) for detailed documentation
- Check [examples/integration.md](examples/integration.md) for integration guides
- Customize the Job struct for your domain
- Add custom statistics and metrics
- Extend keyboard shortcuts

## Support

For issues or questions:

1. Check the troubleshooting section
2. Review the integration guide
3. Examine the source code comments
4. Adapt the code to your needs

Happy monitoring! 🚀
