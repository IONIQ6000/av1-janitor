# Integration Guide

This guide shows how to integrate the portable TUI into your Rust project.

## Quick Start

### 1. Add to existing project

If you have an existing Rust project:

```bash
# Copy the TUI package into your project
cp -r tui-package/src/models your-project/src/
cp tui-package/src/main.rs your-project/src/bin/tui.rs

# Add dependencies to your Cargo.toml
cat tui-package/Cargo.toml >> your-project/Cargo.toml
```

### 2. Standalone binary

To use as a standalone binary:

```bash
cd tui-package
cargo build --release
./target/release/tui --job-state-dir /path/to/jobs
```

## Adapting to Your Domain

### Example: Task Management System

Let's say you're building a task management system. Here's how to adapt the TUI:

#### 1. Modify the Job struct

Edit `src/models/job.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: JobStatus,
    
    // Your domain-specific fields
    pub task_name: String,
    pub task_description: String,
    pub assigned_to: Option<String>,
    pub priority: TaskPriority,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}
```

#### 2. Update table columns

Edit `src/main.rs` to show your fields:

```rust
enum TableColumn {
    Status,
    TaskName,
    Priority,
    AssignedTo,
    Tags,
    CreatedAt,
    Duration,
}

impl TableColumn {
    fn header(&self) -> &'static str {
        match self {
            TableColumn::Status => "STATUS",
            TableColumn::TaskName => "TASK",
            TableColumn::Priority => "PRIORITY",
            TableColumn::AssignedTo => "ASSIGNED",
            TableColumn::Tags => "TAGS",
            TableColumn::CreatedAt => "CREATED",
            TableColumn::Duration => "DURATION",
        }
    }
}
```

#### 3. Update rendering logic

Modify the table rendering to display your fields:

```rust
// In the render_job_table function
for job in filtered_jobs {
    let row_cells = vec![
        // Status
        Cell::from(Span::styled(
            status_symbol(job.status),
            Style::default().fg(color_scheme.status_color(&job.status))
        )),
        
        // Task name
        Cell::from(job.task_name.clone()),
        
        // Priority
        Cell::from(Span::styled(
            format!("{:?}", job.priority),
            Style::default().fg(priority_color(&job.priority))
        )),
        
        // Assigned to
        Cell::from(job.assigned_to.as_deref().unwrap_or("-")),
        
        // Tags
        Cell::from(job.tags.join(", ")),
        
        // Created at
        Cell::from(format_timestamp(&job.created_at)),
        
        // Duration
        Cell::from(format_duration(job)),
    ];
    
    rows.push(Row::new(row_cells));
}
```

### Example: Build System

For a build system that compiles projects:

#### 1. Adapt Job struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    
    // Build-specific fields
    pub project_name: String,
    pub project_path: PathBuf,
    pub build_target: String,
    pub compiler: String,
    pub optimization_level: String,
    pub warnings_count: Option<u32>,
    pub errors_count: Option<u32>,
    pub artifact_size: Option<u64>,
}
```

#### 2. Track build progress

```rust
struct JobProgress {
    // Build-specific progress
    files_compiled: u32,
    total_files: u32,
    current_file: String,
    compilation_rate: f64,  // files per second
    estimated_completion: Option<DateTime<Utc>>,
}

impl JobProgress {
    fn calculate_progress(&self) -> f64 {
        if self.total_files > 0 {
            (self.files_compiled as f64 / self.total_files as f64) * 100.0
        } else {
            0.0
        }
    }
}
```

#### 3. Update statistics

```rust
struct StatisticsCache {
    total_builds: u64,
    successful_builds: u64,
    failed_builds: u64,
    average_build_time: f64,
    total_warnings: u64,
    total_errors: u64,
    build_success_rate: f64,
}
```

## Custom Storage Backend

If you don't use JSON files for storage:

### Database Backend

```rust
use sqlx::{Pool, Postgres};

pub async fn load_all_jobs(pool: &Pool<Postgres>) -> Result<Vec<Job>> {
    let jobs = sqlx::query_as!(
        Job,
        "SELECT * FROM jobs ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;
    
    Ok(jobs)
}

pub async fn save_job(job: &Job, pool: &Pool<Postgres>) -> Result<()> {
    sqlx::query!(
        "INSERT INTO jobs (id, status, created_at, ...) VALUES ($1, $2, $3, ...)",
        job.id,
        job.status as _,
        job.created_at,
        // ... other fields
    )
    .execute(pool)
    .await?;
    
    Ok(())
}
```

### Redis Backend

```rust
use redis::{Client, Commands};

pub fn load_all_jobs(client: &Client) -> Result<Vec<Job>> {
    let mut con = client.get_connection()?;
    let keys: Vec<String> = con.keys("job:*")?;
    
    let mut jobs = Vec::new();
    for key in keys {
        let json: String = con.get(&key)?;
        let job: Job = serde_json::from_str(&json)?;
        jobs.push(job);
    }
    
    Ok(jobs)
}

pub fn save_job(job: &Job, client: &Client) -> Result<()> {
    let mut con = client.get_connection()?;
    let json = serde_json::to_string(job)?;
    con.set(format!("job:{}", job.id), json)?;
    Ok(())
}
```

## Customizing the UI

### Change refresh rate

```rust
// In main() function
let tick_rate = Duration::from_millis(100); // Faster refresh (default: 250ms)
```

### Add custom keyboard shortcuts

```rust
// In handle_input() function
match key.code {
    KeyCode::Char('c') => {
        // Custom action: cancel job
        app.cancel_selected_job()?;
    }
    KeyCode::Char('p') => {
        // Custom action: pause job
        app.pause_selected_job()?;
    }
    // ... other shortcuts
}
```

### Modify color scheme

```rust
impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            // Status colors
            pending: Color::Rgb(255, 200, 0),    // Custom orange
            running: Color::Rgb(0, 200, 255),    // Custom cyan
            success: Color::Rgb(0, 255, 100),    // Custom green
            failed: Color::Rgb(255, 50, 50),     // Custom red
            
            // UI colors
            border_normal: Color::Rgb(100, 100, 100),
            border_selected: Color::Rgb(0, 150, 255),
            // ... other colors
        }
    }
}
```

### Add custom panels

```rust
fn render_custom_panel(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title("Custom Panel")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    
    let content = Paragraph::new(vec![
        Line::from("Custom metric 1: 42"),
        Line::from("Custom metric 2: 100%"),
    ])
    .block(block);
    
    f.render_widget(content, area);
}

// Add to main render function
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3),      // Header
        Constraint::Length(5),      // Custom panel
        Constraint::Min(10),        // Table
        Constraint::Length(3),      // Status bar
    ])
    .split(f.size());

render_header(f, chunks[0], app);
render_custom_panel(f, chunks[1], app);  // Your custom panel
render_job_table(f, chunks[2], app);
render_status_bar(f, chunks[3], app);
```

## Testing

### Unit tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_job_filtering() {
        let mut app = App::new(PathBuf::from("/tmp"), PathBuf::from("/tmp"));
        app.jobs = vec![
            Job { status: JobStatus::Pending, ..Default::default() },
            Job { status: JobStatus::Running, ..Default::default() },
            Job { status: JobStatus::Success, ..Default::default() },
        ];
        
        app.ui_state.filter = JobFilter::Running;
        let filtered = app.filter_jobs(&app.jobs);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].status, JobStatus::Running);
    }
    
    #[test]
    fn test_progress_calculation() {
        let progress = JobProgress {
            temp_file_size: 500_000_000,
            original_size: 1_000_000_000,
            ..Default::default()
        };
        
        assert_eq!(progress.progress_percent, 50.0);
    }
}
```

### Integration tests

```rust
#[tokio::test]
async fn test_full_workflow() {
    let temp_dir = tempfile::tempdir().unwrap();
    let job_dir = temp_dir.path().join("jobs");
    std::fs::create_dir_all(&job_dir).unwrap();
    
    // Create test job
    let job = Job::new(PathBuf::from("/test/file.mkv"));
    save_job(&job, &job_dir).unwrap();
    
    // Load jobs
    let jobs = load_all_jobs(&job_dir).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, job.id);
}
```

## Performance Tips

1. **Limit job list size**: Only load recent jobs (last 1000)
2. **Cache expensive calculations**: Use `StatisticsCache` pattern
3. **Debounce file I/O**: Don't reload jobs on every frame
4. **Use efficient data structures**: HashMap for O(1) lookups
5. **Minimize allocations**: Reuse buffers where possible

## Troubleshooting

### TUI not rendering correctly

- Ensure terminal supports 256 colors: `echo $TERM`
- Try setting: `export TERM=xterm-256color`

### High CPU usage

- Increase tick rate: `Duration::from_millis(500)`
- Reduce refresh frequency for expensive operations

### Jobs not updating

- Check file permissions on job directory
- Verify JSON format matches Job struct
- Enable debug logging to see errors

## Examples

See the `examples/` directory for complete working examples:

- `task_manager.rs` - Task management system
- `build_system.rs` - Build/compilation system
- `data_pipeline.rs` - Data processing pipeline
- `test_runner.rs` - Test execution system

## Contributing

Feel free to extend and modify this TUI for your needs. Key extension points:

- `Job` struct - Add your domain fields
- `TableColumn` enum - Define your columns
- `StatisticsCache` - Track your metrics
- `ColorScheme` - Customize appearance
- Keyboard handlers - Add your shortcuts

Happy coding!
