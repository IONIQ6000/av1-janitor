# Design Document

## Overview

This design addresses missing information issues in the AV1 transcoding TUI by ensuring all available data is properly displayed, missing data is clearly indicated, and the UI gracefully handles incomplete metadata. The solution focuses on improving data validation, display logic, and user feedback when information is unavailable.

## Architecture

The TUI follows a Model-View architecture:

1. **Data Layer**: Job state files contain video metadata, progress information, and job history
2. **Application State**: `App` struct maintains jobs, progress tracking, statistics cache, and UI state
3. **Rendering Layer**: Multiple render functions display different UI components
4. **Update Loop**: Periodic refresh updates job data and recalculates derived information

The key architectural principle is that **missing data should be explicitly handled** rather than silently omitted or showing incorrect values.

## Components and Interfaces

### 1. Job Data Validation

**Purpose**: Validate that job data contains expected fields and identify missing metadata

**Interface**:
```rust
// Check if job has all metadata needed for savings estimation
fn has_estimation_metadata(job: &Job) -> bool

// Check if job has complete video metadata
fn has_complete_video_metadata(job: &Job) -> bool

// Get list of missing metadata fields for a job
fn get_missing_metadata_fields(job: &Job) -> Vec<&'static str>

// Validate that progress tracking has required fields
fn validate_progress_data(progress: &JobProgress) -> bool
```

### 2. Display Formatting

**Purpose**: Format data consistently with clear indicators for missing values

**Interface**:
```rust
// Format optional value with fallback
fn format_optional<T: Display>(value: Option<T>, fallback: &str) -> String

// Format file size with fallback
fn format_size_optional(bytes: Option<u64>) -> String

// Format duration with fallback
fn format_duration_optional(start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>) -> String

// Format percentage with fallback
fn format_percentage_optional(value: Option<f64>) -> String

// Format missing metadata indicator
fn format_missing_metadata(missing_fields: &[&str]) -> String
```

### 3. Progress Calculation

**Purpose**: Calculate progress metrics with validation and fallback values

**Interface**:
```rust
// Calculate progress percentage with validation
fn calculate_progress_percentage(current: u64, estimated_total: u64) -> Option<f64>

// Calculate ETA with validation
fn calculate_eta(bytes_remaining: u64, bytes_per_second: f64) -> Option<DateTime<Utc>>

// Calculate FPS with validation
fn calculate_fps(progress_delta: f64, time_delta: f64, total_frames: Option<u64>) -> Option<f64>

// Validate progress values are within reasonable bounds
fn validate_progress_values(progress: &JobProgress) -> bool
```

### 4. Statistics Aggregation

**Purpose**: Calculate aggregate statistics with proper handling of incomplete data

**Interface**:
```rust
// Calculate statistics from job list
impl StatisticsCache {
    fn calculate(jobs: &[Job]) -> Self
    fn needs_refresh(&self) -> bool
}

// Calculate total space saved (only from completed jobs)
fn calculate_total_space_saved(jobs: &[Job]) -> u64

// Calculate average compression ratio (only from successful jobs)
fn calculate_average_compression_ratio(jobs: &[Job]) -> f64

// Calculate estimated pending savings (only from jobs with metadata)
fn calculate_estimated_pending_savings(jobs: &[Job], cache: &HashMap<String, Option<(f64, f64)>>) -> u64
```

## Data Models

### Job Metadata Completeness

Jobs can have varying levels of metadata completeness:

1. **Minimal**: Only file path and status (newly created jobs)
2. **Partial**: Some video metadata extracted (background extraction in progress)
3. **Complete**: All video metadata available (ready for estimation/transcoding)

The UI must handle all three states gracefully.

### Progress Tracking States

Progress tracking has multiple states:

1. **Not Started**: Job is pending, no progress tracking
2. **Probing**: Job started, no temp file yet (< 30 seconds)
3. **Transcoding**: Temp file exists and growing
4. **Verifying**: Temp file complete (> 95% progress)
5. **Replacing**: Original file being replaced
6. **Complete**: Job finished

Each state requires different display logic.

### Missing Data Indicators

The system uses consistent indicators for missing data:

- `-`: Data not available or not applicable
- `(not available)`: Metadata not yet extracted
- `(not set)`: Configuration value not specified
- `(not started)`: Job hasn't started yet
- `(not finished)`: Job hasn't completed yet
- `-orig,codec,w,h`: Specific missing fields for estimation

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Metadata field display completeness
*For any* job with video metadata fields (resolution, codec, bitrate, HDR status, bit depth), when displayed in the job table, all available fields should be shown and unavailable fields should display "-".
**Validates: Requirements 1.1**

### Property 2: Missing metadata indicator accuracy
*For any* job lacking metadata required for estimation, the displayed indicator should list exactly the missing field names (orig, codec, w, h, br, fps).
**Validates: Requirements 1.2**

### Property 3: Detail view metadata completeness
*For any* job in detail view, all metadata sections (file paths, status, job history, video metadata, encoding parameters, file sizes) should be present, with each field showing either its value or "(not available)" / "(not set)".
**Validates: Requirements 1.3, 6.1, 6.6**

### Property 4: Codec name formatting consistency
*For any* job with a codec name, the displayed codec should be in uppercase format.
**Validates: Requirements 1.5**

### Property 5: Running job progress display completeness
*For any* running job, the display should include stage, progress percentage (0-100), speed (non-negative or "-"), ETA (future time or "-"), and elapsed time.
**Validates: Requirements 2.1, 2.5**

### Property 6: FPS calculation validity
*For any* running job with frame rate metadata, if FPS can be calculated it should be between 0.1 and 500, otherwise "-" should be displayed.
**Validates: Requirements 2.2**

### Property 7: Running job estimation display
*For any* running job with complete metadata, estimated final size and compression ratio should be calculated and displayed.
**Validates: Requirements 2.3**

### Property 8: Quality setting display
*For any* job with an av1_quality value, the quality (CRF) should be displayed in the current job panel.
**Validates: Requirements 2.4**

### Property 9: Completed job timing completeness
*For any* completed job, the detail view should display created time, started time, finished time, queue time, processing time, and total time.
**Validates: Requirements 3.1**

### Property 10: Pending job timing display
*For any* pending job, the detail view should display created time and "(not started)" for start/finish times.
**Validates: Requirements 3.2**

### Property 11: Running job timing display
*For any* running job, the display should show created time, started time, and current elapsed time.
**Validates: Requirements 3.3**

### Property 12: Duration formatting consistency
*For any* duration value, the format should consistently use "Xh Ym Zs", "Ym Zs", or "Zs" notation based on magnitude.
**Validates: Requirements 3.4**

### Property 13: Missing timing indicator accuracy
*For any* job with missing started_at, the display should show "(not started)", and for missing finished_at, it should show "(not finished)".
**Validates: Requirements 3.5**

### Property 14: Savings estimation with complete metadata
*For any* pending job with all required metadata (original_bytes, video_codec, video_width, video_height, video_bitrate, video_frame_rate), estimated savings in GB and percentage should be calculated and displayed.
**Validates: Requirements 4.1**

### Property 15: Missing metadata field indication
*For any* pending job lacking metadata for estimation, the display should show which specific fields are missing (e.g., "-orig,codec,w").
**Validates: Requirements 4.2**

### Property 16: Actual savings preference
*For any* completed job with both original_bytes and new_bytes, actual savings should be displayed rather than estimates.
**Validates: Requirements 4.3**

### Property 17: Quality-based estimation
*For any* job with av1_quality set, the estimation should use quality-based calculation rather than codec-only estimation.
**Validates: Requirements 4.4**

### Property 18: Estimate prefix consistency
*For any* displayed estimated savings value, it should be prefixed with "~" to indicate it is an estimate.
**Validates: Requirements 4.5**

### Property 19: Statistics dashboard completeness
*For any* job set, the statistics dashboard should display total space saved, average compression ratio, total processing time, and success rate.
**Validates: Requirements 5.1**

### Property 20: Pending savings inclusion
*For any* job set with pending jobs that have complete metadata, the statistics should include estimated pending savings.
**Validates: Requirements 5.2**

### Property 21: Zero statistics display
*For any* job set with no completed jobs, the statistics should display zero values rather than hiding the dashboard.
**Validates: Requirements 5.3**

### Property 22: Trend sparkline data
*For any* job set with completed jobs, the trend sparklines should use data from the most recent 20 completed jobs.
**Validates: Requirements 5.4**

### Property 23: Encoding parameters display
*For any* job with encoding parameters set (av1_quality, av1_profile, is_web_like), the detail view should display all set parameters.
**Validates: Requirements 6.2**

### Property 24: Dual format file size display
*For any* job with file size data, the detail view should display both human-readable format (e.g., "2.5 GB") and exact byte count.
**Validates: Requirements 6.3**

### Property 25: Compression calculation accuracy
*For any* completed job with original_bytes and new_bytes, the displayed space saved should equal (original_bytes - new_bytes) and compression ratio should equal ((original_bytes - new_bytes) / original_bytes * 100).
**Validates: Requirements 6.4**

### Property 26: Narrow terminal column visibility
*For any* terminal width less than 80 columns, the job table should display only status, file name, and savings columns.
**Validates: Requirements 7.1**

### Property 27: Short terminal component visibility
*For any* terminal height less than 20 lines, the statistics dashboard should be hidden.
**Validates: Requirements 7.2**

### Property 28: Very small terminal simplified view
*For any* terminal smaller than 80x12, a simplified view with a clear message should be displayed.
**Validates: Requirements 7.3**

### Property 29: Column priority in constrained layouts
*For any* terminal with width constraints, the visible columns should prioritize status, file name, and savings information.
**Validates: Requirements 7.5**

### Property 30: Activity status display accuracy
*For any* system state, if running jobs exist the status should show "Processing", otherwise "Idle".
**Validates: Requirements 8.1, 8.4, 8.5**

## Error Handling

### Missing Metadata Handling

When video metadata is missing:
1. Display clear indicator showing which fields are missing
2. Disable estimation calculations that require missing fields
3. Continue displaying other available information
4. Update display when metadata becomes available

### Invalid Progress Data

When progress data is invalid or inconsistent:
1. Validate all calculated values are within reasonable bounds
2. Fall back to "-" for values that cannot be calculated
3. Log warnings for debugging but don't crash the UI
4. Continue displaying other valid progress information

### Statistics Calculation Errors

When statistics cannot be calculated:
1. Use zero values for missing data points
2. Skip jobs with incomplete data in aggregations
3. Display statistics even if some values are zero
4. Refresh statistics when new data becomes available

## Testing Strategy

### Unit Testing

Unit tests will verify:
- Metadata validation functions correctly identify missing fields
- Display formatting functions handle None values correctly
- Progress calculation functions validate inputs and return None for invalid data
- Statistics aggregation functions skip incomplete jobs
- Duration formatting handles all edge cases (< 1 minute, < 1 hour, > 1 day)

### Property-Based Testing

Property-based tests will use the `proptest` crate (already used in the project) with a minimum of 100 iterations per test.

Each property-based test will be tagged with a comment referencing the correctness property using this format: `**Feature: tui-missing-info-fix, Property {number}: {property_text}**`

Property tests will verify:
- **Property 1**: Metadata display completeness across random job configurations
- **Property 2**: Progress information consistency with random progress states
- **Property 3**: Timing information completeness with random timestamps
- **Property 4**: Savings estimation accuracy with random metadata combinations
- **Property 5**: Statistics calculation correctness with random job sets
- **Property 6**: Detail view information completeness with random jobs
- **Property 7**: Responsive layout adaptation with random terminal sizes
- **Property 8**: System resource display accuracy with random system states

### Integration Testing

Integration tests will verify:
- Complete refresh cycle updates all UI components correctly
- Missing metadata is handled consistently across all views
- Progress tracking updates correctly as temp files grow
- Statistics cache refreshes when stale
- Layout recalculation works correctly on terminal resize

## Implementation Notes

### Existing Code Analysis

The current TUI implementation already has:
- Comprehensive rendering functions for all UI components
- Progress tracking with JobProgress struct
- Statistics caching with StatisticsCache
- Responsive layout with LayoutConfig
- Metadata display in job table and detail view

### Issues Identified

Based on code review, potential issues include:

1. **Inconsistent Missing Data Handling**: Some functions return "-" while others return empty strings or omit fields
2. **No Validation of Progress Values**: Progress percentage could exceed 100% or be negative
3. **Missing Metadata Not Clearly Indicated**: Users can't tell why estimation fails
4. **Stale Progress Data**: No validation that progress data is recent
5. **Incomplete Error Handling**: Some calculations could panic on invalid data

### Required Changes

The implementation will need to:

1. Add validation functions for job metadata completeness
2. Standardize missing data indicators across all views
3. Add bounds checking for all calculated progress values
4. Improve error messages for missing metadata
5. Add validation for timing information consistency
6. Ensure all optional fields are explicitly handled
7. Add unit tests for all validation and formatting functions
8. Add property-based tests for correctness properties

## Performance Considerations

- Metadata validation should be cached to avoid repeated checks
- Statistics calculation should only run when cache is stale (> 5 seconds)
- Progress tracking should only update for running jobs
- Layout recalculation should only occur on terminal resize
- Estimated savings cache should be cleaned up for completed jobs

## Security Considerations

No security implications - this is a read-only monitoring interface.

## Future Enhancements

- Scrollable detail view for very long job information
- Configurable column visibility in job table
- Export job statistics to CSV
- Real-time log streaming for running jobs
- Filtering by multiple criteria simultaneously
