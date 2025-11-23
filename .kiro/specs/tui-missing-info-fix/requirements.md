# Requirements Document

## Introduction

This specification addresses missing information issues in the AV1 transcoding TUI (Terminal User Interface). The TUI displays job status, video metadata, encoding progress, and statistics. Users have reported that information is missing "all over the place," which impacts their ability to monitor transcoding jobs effectively.

## Glossary

- **TUI**: Terminal User Interface - the text-based interface for monitoring AV1 transcoding jobs
- **Job**: A transcoding task that converts a video file to AV1 format
- **Metadata**: Video file properties including resolution, codec, bitrate, HDR status, bit depth, frame rate
- **Job State**: The current status of a job (Pending, Running, Success, Failed, Skipped)
- **Progress Tracking**: Real-time monitoring of encoding progress including percentage, ETA, speed
- **Statistics Cache**: Aggregated metrics across all jobs including space saved, compression ratios, success rates
- **Detail View**: Modal overlay showing comprehensive information about a selected job
- **Job Table**: Main table displaying all jobs with their key properties

## Requirements

### Requirement 1

**User Story:** As a user monitoring transcoding jobs, I want to see complete video metadata for all jobs, so that I can understand what is being transcoded and make informed decisions.

#### Acceptance Criteria

1. WHEN a job is displayed in the job table THEN the system SHALL show resolution, codec, bitrate, HDR status, and bit depth when available
2. WHEN video metadata is not yet extracted THEN the system SHALL display a clear indicator showing which specific metadata fields are missing
3. WHEN a job is in the detail view THEN the system SHALL display all available video metadata including frame rate, pixel format, source and target bit depths
4. WHEN metadata becomes available after background extraction THEN the system SHALL update the display to show the newly available information
5. WHEN displaying codec information THEN the system SHALL use consistent uppercase formatting for codec names

### Requirement 2

**User Story:** As a user monitoring running jobs, I want to see detailed progress information, so that I can estimate completion time and identify stuck jobs.

#### Acceptance Criteria

1. WHEN a job is running THEN the system SHALL display current stage, progress percentage, speed, ETA, and elapsed time
2. WHEN a job is running THEN the system SHALL display current FPS processing rate when calculable
3. WHEN a job is running THEN the system SHALL display estimated final size and compression ratio
4. WHEN a job is running THEN the system SHALL display the quality setting (CRF value) being used for encoding
5. WHEN progress information cannot be calculated THEN the system SHALL display "-" rather than showing stale or incorrect values

### Requirement 3

**User Story:** As a user reviewing job history, I want to see complete timing information, so that I can understand how long jobs took and identify performance issues.

#### Acceptance Criteria

1. WHEN a job has completed THEN the system SHALL display created time, started time, finished time, queue time, processing time, and total time
2. WHEN a job is pending THEN the system SHALL display created time and indicate that start/finish times are not yet available
3. WHEN a job is running THEN the system SHALL display created time, started time, and current elapsed time
4. WHEN displaying durations THEN the system SHALL format them consistently using hours, minutes, and seconds notation
5. WHEN timing information is missing THEN the system SHALL display "(not started)" or "(not finished)" rather than showing incorrect values

### Requirement 4

**User Story:** As a user estimating space savings, I want to see accurate savings estimates for pending jobs, so that I can prioritize which jobs to run.

#### Acceptance Criteria

1. WHEN a pending job has complete metadata THEN the system SHALL calculate and display estimated space savings in GB and percentage
2. WHEN a pending job lacks metadata for estimation THEN the system SHALL display which specific metadata fields are missing
3. WHEN a completed job has actual savings THEN the system SHALL display actual savings rather than estimates
4. WHEN the quality setting is available THEN the system SHALL use quality-based estimation for more accurate predictions
5. WHEN displaying estimated savings THEN the system SHALL prefix values with "~" to indicate they are estimates

### Requirement 5

**User Story:** As a user viewing the statistics dashboard, I want to see accurate aggregate metrics, so that I can understand overall transcoding performance.

#### Acceptance Criteria

1. WHEN the statistics dashboard is displayed THEN the system SHALL show total space saved, average compression ratio, total processing time, and success rate
2. WHEN there are pending jobs with metadata THEN the system SHALL display estimated pending savings
3. WHEN there are no completed jobs THEN the system SHALL display zero values rather than hiding the statistics
4. WHEN displaying trend sparklines THEN the system SHALL show recent processing times and compression ratios for the last 20 jobs
5. WHEN statistics are stale THEN the system SHALL refresh them automatically every 5 seconds

### Requirement 6

**User Story:** As a user viewing job details, I want to see all available information in one place, so that I can troubleshoot issues and verify job configuration.

#### Acceptance Criteria

1. WHEN the detail view is opened THEN the system SHALL display file paths, status, reason, job history, video metadata, encoding parameters, and file sizes
2. WHEN encoding parameters are set THEN the system SHALL display AV1 quality, profile, and web-like content flag
3. WHEN file sizes are available THEN the system SHALL display both human-readable format and exact byte counts
4. WHEN compression has occurred THEN the system SHALL calculate and display space saved and compression ratio
5. WHEN information is not available THEN the system SHALL display "(not available)" or "(not set)" rather than omitting the field

### Requirement 7

**User Story:** As a user with a small terminal, I want the TUI to adapt gracefully, so that I can still monitor jobs even with limited screen space.

#### Acceptance Criteria

1. WHEN the terminal width is less than 80 columns THEN the system SHALL display only essential columns in the job table
2. WHEN the terminal height is less than 20 lines THEN the system SHALL hide the statistics dashboard to prioritize job information
3. WHEN the terminal is very small THEN the system SHALL display a simplified view with a clear message about limited space
4. WHEN the terminal is resized THEN the system SHALL recalculate the layout and adjust visible components
5. WHEN columns are hidden due to space constraints THEN the system SHALL prioritize status, file name, and savings information

### Requirement 8

**User Story:** As a user monitoring the system, I want to see current system resource usage, so that I can understand if the system is under load.

#### Acceptance Criteria

1. WHEN the top bar is displayed THEN the system SHALL show current activity status with appropriate icon
2. WHEN the top bar is displayed THEN the system SHALL show CPU usage percentage
3. WHEN the top bar is displayed THEN the system SHALL show memory usage in GB and percentage
4. WHEN no jobs are running THEN the system SHALL display "Idle" status
5. WHEN jobs are running THEN the system SHALL display "Processing" status with appropriate color coding
