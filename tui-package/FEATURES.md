# Feature List

Complete list of features included in this portable TUI package.

## Core Features

### Job Management

- ✅ **Real-time job monitoring** - Watch jobs update live
- ✅ **Status tracking** - Pending, Running, Success, Failed, Skipped
- ✅ **Job filtering** - Filter by status (All, Pending, Running, Success, Failed)
- ✅ **Job sorting** - Sort by Date, Size, Status, or Savings
- ✅ **Job selection** - Navigate with keyboard (vim-style bindings)
- ✅ **Detail view** - Expand selected job to see full details
- ✅ **Job requeue** - Requeue running jobs with 'r' key

### Progress Tracking

- ✅ **Real-time progress** - Track job progress percentage
- ✅ **ETA calculation** - Estimate time to completion
- ✅ **Stage detection** - Detect job stages (Probing, Transcoding, Verifying, Replacing)
- ✅ **Speed tracking** - Monitor bytes/second processing rate
- ✅ **Frame tracking** - Track frames processed and FPS (when available)
- ✅ **Compression ratio** - Real-time compression ratio calculation
- ✅ **File size estimation** - Estimate final output size

### System Metrics

- ✅ **CPU usage** - System-wide CPU utilization
- ✅ **Memory usage** - RAM usage with total/used display
- ✅ **GPU usage** - Intel Arc GPU utilization (extensible for other GPUs)
- ✅ **Process monitoring** - Track system resource usage

### Statistics Dashboard

- ✅ **Aggregate metrics** - Total space saved, average compression ratio
- ✅ **Success rate** - Percentage of successful jobs
- ✅ **Processing time** - Total and average processing time
- ✅ **Estimated savings** - Predicted savings for pending jobs
- ✅ **Trend analysis** - Recent processing times and compression ratios
- ✅ **Sparkline graphs** - Visual representation of trends
- ✅ **Statistics caching** - Efficient calculation with 5-second cache

### User Interface

- ✅ **Responsive layout** - Adapts to terminal size (80-200+ columns)
- ✅ **Color-coded status** - Visual status indicators
- ✅ **Table view** - Scrollable job list with multiple columns
- ✅ **Header panel** - System metrics at a glance
- ✅ **Status bar** - Current filter, sort mode, and help
- ✅ **Message system** - Temporary messages with timeout
- ✅ **Smooth scrolling** - Page up/down navigation
- ✅ **Selection highlighting** - Clear visual selection

### Keyboard Controls

- ✅ **Navigation** - Arrow keys, vim bindings (j/k), page up/down
- ✅ **Filtering** - 'f' to cycle filters
- ✅ **Sorting** - 's' to cycle sort modes
- ✅ **Detail view** - Enter to toggle
- ✅ **Refresh** - 'R' to force refresh
- ✅ **Requeue** - 'r' to requeue running job
- ✅ **Quit** - 'q' to exit

## Display Features

### Table Columns (Responsive)

**Large terminals (160+ cols):**
- Status indicator
- File name
- Resolution
- Codec
- Bitrate
- HDR flag
- Bit depth
- Original size
- New size
- Compression ratio
- Quality setting
- Estimated savings
- Processing time
- Reason (for failures/skips)

**Medium terminals (120-159 cols):**
- Status
- File name
- Resolution
- Codec
- Original size
- New size
- Compression ratio
- Savings
- Time

**Small terminals (80-119 cols):**
- Status
- File name
- Original size
- New size
- Savings

**Very small terminals (<80 cols):**
- Status
- File name
- Savings

### Current Job Panel

When a job is running, displays:
- Job file name
- Progress bar with percentage
- Current stage (Probing, Transcoding, Verifying, Replacing)
- Processing speed (MB/s)
- ETA (estimated time remaining)
- Original size vs current temp file size
- Compression ratio (real-time)
- Frames processed / total frames
- Current FPS

### Statistics Panel

Displays aggregate metrics:
- Total space saved (GB)
- Average compression ratio (%)
- Total processing time
- Estimated pending savings
- Success rate (%)
- Recent processing times (sparkline)
- Recent compression ratios (sparkline)
- Recent completion rate

### Detail View

When Enter is pressed on a job, shows:
- Full file path
- Complete metadata (resolution, codec, bitrate, etc.)
- Timestamps (created, started, finished)
- Processing duration
- Size comparison (before/after)
- Compression statistics
- Quality settings used
- Failure reason (if applicable)
- HDR information
- Bit depth details

## Technical Features

### Performance

- ✅ **Efficient rendering** - Only updates changed components
- ✅ **Statistics caching** - Avoid expensive recalculations
- ✅ **Lazy loading** - Load jobs on demand
- ✅ **Debounced I/O** - Minimize file system operations
- ✅ **Configurable refresh rate** - Balance responsiveness vs CPU usage

### Data Management

- ✅ **JSON storage** - Simple file-based job storage
- ✅ **Atomic writes** - Safe job state updates
- ✅ **Error handling** - Graceful degradation on errors
- ✅ **Data validation** - Validate job JSON on load
- ✅ **Extensible schema** - Easy to add custom fields

### Customization

- ✅ **Color scheme** - Fully customizable colors
- ✅ **Column configuration** - Add/remove table columns
- ✅ **Layout configuration** - Adjust panel sizes
- ✅ **Keyboard bindings** - Add custom shortcuts
- ✅ **Statistics** - Track custom metrics
- ✅ **Progress tracking** - Adapt to your job types

### Integration

- ✅ **Standalone binary** - Run as independent program
- ✅ **Library integration** - Embed in existing projects
- ✅ **CLI arguments** - Configure via command line
- ✅ **Environment variables** - Support for env config
- ✅ **Pluggable storage** - Easy to swap storage backend

## Responsive Design

### Terminal Size Adaptation

| Size | Width | Features |
|------|-------|----------|
| Very Small | <80 cols | Minimal columns, no statistics |
| Small | 80-119 cols | Essential columns, compact layout |
| Medium | 120-159 cols | Most columns, statistics panel |
| Large | 160+ cols | All columns, full statistics |

### Component Visibility

- **Statistics panel**: Hidden on terminals <20 lines
- **Current job panel**: Hidden on terminals <15 lines
- **Detail view**: Replaces table when active
- **Status bar**: Always visible (minimum 2 lines)
- **Header**: Always visible (3 lines)

## Color Coding

### Status Colors

- 🟡 **Pending** - Yellow
- 🟢 **Running** - Green
- 🔵 **Success** - Blue
- 🔴 **Failed** - Red
- ⚫ **Skipped** - Gray

### Codec Colors

- 🟡 **H.264/AVC** - Yellow
- 🟢 **HEVC/H.265** - Green
- 🔵 **AV1** - Blue
- 🔵 **VP9** - Cyan
- ⚫ **Other** - Gray

### Progress Colors

- 🟡 **Probing** - Yellow
- 🟢 **Transcoding** - Green
- 🔵 **Verifying** - Cyan
- 🔵 **Complete** - Blue

### Metric Colors

- 🟢 **Low** - Green (good)
- 🟡 **Medium** - Yellow (warning)
- 🔴 **High** - Red (critical)

## Extensibility

### Easy to Extend

- Add custom job fields
- Add custom table columns
- Add custom statistics
- Add custom keyboard shortcuts
- Add custom panels
- Add custom color schemes
- Add custom progress tracking
- Add custom storage backends

### Well-Documented

- Inline code comments
- Architecture documentation
- Integration guides
- Example implementations
- Troubleshooting guides

## Platform Support

- ✅ **Linux** - Full support
- ✅ **macOS** - Full support
- ✅ **Windows** - Full support (with crossterm)
- ✅ **WSL** - Full support

## Dependencies

All dependencies are stable, well-maintained crates:

- `ratatui` - Terminal UI framework
- `crossterm` - Cross-platform terminal manipulation
- `tokio` - Async runtime
- `serde` - Serialization framework
- `chrono` - Date/time handling
- `anyhow` - Error handling
- `sysinfo` - System metrics
- `humansize` - Human-readable sizes
- `clap` - CLI argument parsing
- `uuid` - Unique identifiers

## Future Enhancement Ideas

These features are not included but could be added:

- [ ] Mouse support (click to select)
- [ ] Search/filter by text
- [ ] Export to CSV/JSON
- [ ] Job history graphs
- [ ] Multi-select operations
- [ ] Job priority management
- [ ] Notification system
- [ ] Log viewer
- [ ] Configuration file support
- [ ] Theme system
- [ ] Plugin architecture
- [ ] Remote monitoring (network)
- [ ] Job scheduling
- [ ] Resource limits
- [ ] Job dependencies

## License

MIT License - Free to use, modify, and distribute.

## Credits

Built with:
- Ratatui - Terminal UI framework
- Crossterm - Terminal manipulation
- Rust - Systems programming language

Inspired by:
- htop - System monitoring
- btop - Resource monitor
- lazygit - Git TUI
- k9s - Kubernetes TUI
