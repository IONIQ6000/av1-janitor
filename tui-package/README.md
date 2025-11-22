# Portable TUI Package

This package contains a fully-featured Terminal User Interface (TUI) built with Ratatui that can be ported to other Rust projects.

## Features

- **Real-time job monitoring** with status tracking (Pending, Running, Success, Failed, Skipped)
- **Progress tracking** for running jobs with ETA calculation
- **System metrics** display (CPU, Memory, GPU usage)
- **Statistics dashboard** with aggregate metrics and trends
- **Responsive layout** that adapts to terminal size
- **Filtering and sorting** capabilities
- **Detail view** for individual jobs
- **Color-coded status** indicators
- **Keyboard navigation** with vim-style bindings

## Package Contents

```
tui-package/
├── README.md                 # This file
├── Cargo.toml               # Dependencies configuration
├── src/
│   ├── main.rs              # Complete TUI implementation
│   └── models/
│       ├── job.rs           # Job data structures
│       ├── config.rs        # Configuration structures
│       └── mod.rs           # Module exports
└── examples/
    └── integration.md       # Integration guide
```

## Dependencies

The TUI requires these Rust crates:

- `ratatui` (0.27) - Terminal UI framework
- `crossterm` (0.28) - Terminal manipulation
- `tokio` (1.40) - Async runtime
- `serde` + `serde_json` (1.0) - Serialization
- `chrono` (0.4) - Date/time handling
- `anyhow` (1.0) - Error handling
- `sysinfo` (0.31) - System metrics
- `humansize` (2.1) - Human-readable sizes
- `clap` (4) - CLI argument parsing
- `uuid` (1.10) - Unique identifiers

## Integration Steps

### 1. Copy the source files

Copy the `src/` directory into your project:

```bash
cp -r tui-package/src/* your-project/src/
```

### 2. Add dependencies to your Cargo.toml

```toml
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
```

### 3. Adapt the data models

The TUI expects jobs to be stored as JSON files in a directory. Adapt the `Job` struct in `src/models/job.rs` to match your domain:

- Keep the core fields: `id`, `status`, `created_at`, `started_at`, `finished_at`
- Modify domain-specific fields to match your use case
- Update the `load_all_jobs()` function if you use a different storage mechanism

### 4. Customize the UI

The TUI is highly customizable:

- **Color scheme**: Modify `ColorScheme` struct in `main.rs`
- **Table columns**: Adjust `TableColumn` enum and rendering logic
- **Statistics**: Update `StatisticsCache` to track your metrics
- **Progress tracking**: Adapt `JobProgress` for your job types

### 5. Run the TUI

```bash
cargo run --bin your-tui-name -- --job-state-dir /path/to/jobs
```

## Key Components

### App Structure

The `App` struct is the main state container:

```rust
struct App {
    jobs: Vec<Job>,              // Job list
    system: System,              // System metrics
    ui_state: UiState,           // UI navigation state
    job_progress: HashMap<...>,  // Progress tracking
    statistics_cache: ...,       // Cached statistics
    color_scheme: ColorScheme,   // UI colors
    // ... configuration fields
}
```

### UI State Management

The `UiState` struct tracks navigation and view mode:

```rust
struct UiState {
    selected_index: Option<usize>,  // Selected row
    filter: JobFilter,              // Active filter
    sort_mode: SortMode,            // Sort order
    view_mode: ViewMode,            // Normal/Detail view
    // ...
}
```

### Keyboard Controls

- `q` - Quit
- `↑/k` - Move up
- `↓/j` - Move down
- `PgUp/u` - Page up
- `PgDn/d` - Page down
- `f` - Cycle filter (All/Pending/Running/Success/Failed)
- `s` - Cycle sort mode
- `Enter` - Toggle detail view
- `r` - Requeue running job
- `R` - Refresh

## Customization Examples

### Change the color scheme

```rust
impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            pending: Color::Cyan,      // Change pending color
            running: Color::Magenta,   // Change running color
            success: Color::Green,     // Change success color
            // ... other colors
        }
    }
}
```

### Add custom table columns

```rust
enum TableColumn {
    Status,
    File,
    // Add your custom columns
    Priority,
    Owner,
    Tags,
}
```

### Modify statistics tracking

```rust
struct StatisticsCache {
    // Add your custom metrics
    total_items_processed: u64,
    average_processing_speed: f64,
    custom_metric: f64,
}
```

## Architecture Notes

### Responsive Layout

The TUI uses `LayoutConfig` to adapt to terminal size:

- **Large terminals (160+ cols)**: Show all columns
- **Medium terminals (120-159 cols)**: Show essential columns
- **Small terminals (80-119 cols)**: Show minimal columns
- **Very small (<80 cols)**: Show critical info only

### Progress Tracking

Progress is tracked by monitoring temporary files:

1. Detect temp file creation
2. Monitor file size growth
3. Calculate bytes/second write rate
4. Estimate completion time
5. Detect job stages (Probing, Transcoding, Verifying, etc.)

### Statistics Caching

Statistics are cached and refreshed every 5 seconds to avoid expensive recalculations on every frame.

## Performance Considerations

- **Refresh rate**: 250ms by default (configurable)
- **Statistics cache**: 5-second TTL
- **File I/O**: Minimized with caching
- **Rendering**: Only updates changed components

## Troubleshooting

### Terminal too small

The TUI requires minimum 80x12 terminal size. It will display a warning if smaller.

### GPU metrics not showing

GPU usage detection works for Intel Arc GPUs. For other GPUs, modify the `get_gpu_usage()` function.

### Jobs not loading

Ensure the job state directory exists and contains valid JSON files matching the `Job` struct format.

## License

This TUI package is provided as-is for integration into your projects. Modify as needed.

## Support

For questions or issues, refer to the original project or adapt the code to your needs.
