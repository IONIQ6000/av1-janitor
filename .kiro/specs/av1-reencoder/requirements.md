# Requirements Document

## Introduction

This document specifies requirements for a Rust-based AV1 video re-encoding daemon with a terminal user interface (TUI). The system targets quality-first software AV1 encoding for a 32-core EPYC processor running in a Debian container. The daemon recursively scans media libraries, applies intelligent filtering and classification, re-encodes suitable videos to AV1 using quality-optimized settings, and provides real-time monitoring through a ratatui-based TUI.

## Glossary

- **Daemon**: The background service that scans directories and manages encoding jobs
- **TUI**: Terminal User Interface built with ratatui for monitoring jobs
- **AV1**: AOMedia Video 1 codec, the target output format
- **SVT-AV1**: Scalable Video Technology for AV1, the preferred encoder
- **libaom-av1**: Reference AV1 encoder, fallback option
- **librav1e**: Rust-based AV1 encoder, last resort fallback
- **CRF**: Constant Rate Factor, quality control parameter (lower = higher quality)
- **WebRip**: Video source from web streaming services requiring special handling
- **DiscLike**: Video source from physical media (Blu-ray, DVD)
- **Stable File**: File whose size has not changed for a specified duration
- **Job**: A single video encoding task with associated metadata
- **Sidecar File**: Metadata file stored alongside video files (.av1skip, .why.txt)
- **Size Gate**: Post-encoding validation that rejects outputs exceeding size threshold
- **EPYC**: AMD server processor with many cores (32 in this case)

## Requirements

### Requirement 1

**User Story:** As a system administrator, I want the daemon to verify ffmpeg availability and version at startup, so that encoding operations will not fail due to missing or incompatible dependencies.

#### Acceptance Criteria

1. WHEN the daemon starts THEN the system SHALL execute `ffmpeg -version` and parse the major version number from the output
2. WHEN the parsed ffmpeg major version is less than 8 THEN the system SHALL abort startup and log an error message
3. WHEN the parsed ffmpeg major version is 8 or greater THEN the system SHALL proceed with encoder availability checks
4. WHEN ffmpeg is not found in the system PATH THEN the system SHALL abort startup and log an error message

### Requirement 2

**User Story:** As a system administrator, I want the daemon to verify AV1 encoder availability at startup, so that I know which encoding path will be used.

#### Acceptance Criteria

1. WHEN the daemon starts THEN the system SHALL execute `ffmpeg -hide_banner -encoders` and search for available AV1 encoders
2. WHEN libsvtav1 is detected THEN the system SHALL set SVT-AV1 as the preferred encoder
3. WHEN libsvtav1 is not detected and libaom-av1 is detected THEN the system SHALL set libaom-av1 as the fallback encoder
4. WHEN neither libsvtav1 nor libaom-av1 is detected and librav1e is detected THEN the system SHALL set librav1e as the last resort encoder
5. WHEN no AV1 encoders are detected THEN the system SHALL abort startup and log an error message

### Requirement 3

**User Story:** As a system administrator, I want to configure the daemon through a TOML configuration file, so that I can customize behavior without modifying code.

#### Acceptance Criteria

1. WHEN the daemon starts THEN the system SHALL load configuration from a TOML file
2. WHEN the configuration file specifies library_roots THEN the system SHALL use those directories for scanning
3. WHEN the configuration file specifies min_bytes THEN the system SHALL skip files smaller than this threshold
4. WHEN the configuration file specifies max_size_ratio THEN the system SHALL use this value for the post-encoding size gate
5. WHEN the configuration file specifies scan_interval_secs THEN the system SHALL wait this duration between directory scans
6. WHEN the configuration file specifies max_concurrent_jobs THEN the system SHALL limit parallel encoding operations to this number
7. WHEN the configuration file is missing or invalid THEN the system SHALL use default values and log a warning

### Requirement 4

**User Story:** As a media library owner, I want the daemon to recursively scan configured directories, so that all video files are discovered for potential encoding.

#### Acceptance Criteria

1. WHEN a scan cycle begins THEN the system SHALL recursively traverse all directories specified in library_roots
2. WHEN a file is encountered THEN the system SHALL check if the file extension matches allowed video formats
3. WHEN a file has an allowed extension THEN the system SHALL add it to the candidate list for further processing
4. WHEN a directory is inaccessible THEN the system SHALL log a warning and continue scanning other directories
5. WHEN a scan cycle completes THEN the system SHALL wait scan_interval_secs before starting the next cycle

### Requirement 5

**User Story:** As a media library owner, I want the daemon to skip files that are already marked for skipping, so that processing time is not wasted on files that should not be encoded.

#### Acceptance Criteria

1. WHEN a candidate file is evaluated THEN the system SHALL check for the presence of a `.av1skip` sidecar file
2. WHEN a `.av1skip` sidecar file exists THEN the system SHALL skip the file and not create a job
3. WHEN a `.av1skip` sidecar file does not exist THEN the system SHALL proceed with further evaluation
4. WHEN a file is skipped due to `.av1skip` THEN the system SHALL log the skip reason if a `.why.txt` sidecar exists

### Requirement 6

**User Story:** As a media library owner, I want the daemon to only process stable files, so that files currently being transferred or written are not corrupted.

#### Acceptance Criteria

1. WHEN a candidate file is evaluated THEN the system SHALL record the file size
2. WHEN 10 seconds have elapsed THEN the system SHALL check the file size again
3. WHEN the file size has changed THEN the system SHALL mark the file as unstable and skip it for this scan cycle
4. WHEN the file size has not changed THEN the system SHALL mark the file as stable and proceed with processing
5. WHEN a file is skipped due to instability THEN the system SHALL re-evaluate it in the next scan cycle

### Requirement 7

**User Story:** As a media library owner, I want the daemon to probe video metadata using ffprobe, so that encoding decisions can be based on accurate technical information.

#### Acceptance Criteria

1. WHEN a stable file is ready for processing THEN the system SHALL execute `ffprobe -v quiet -print_format json -show_format -show_streams` on the file
2. WHEN ffprobe completes successfully THEN the system SHALL parse the JSON output and extract video stream metadata
3. WHEN ffprobe fails THEN the system SHALL skip the file, create a `.av1skip` sidecar, and log the failure reason
4. WHEN multiple video streams exist THEN the system SHALL identify the main video stream based on default disposition or stream order
5. WHEN no video streams exist THEN the system SHALL skip the file and create a `.av1skip` sidecar

### Requirement 8

**User Story:** As a media library owner, I want the daemon to skip files that do not meet minimum size requirements, so that small low-bitrate files are not degraded by AV1 encoding.

#### Acceptance Criteria

1. WHEN a file is evaluated THEN the system SHALL compare the file size to the configured min_bytes threshold
2. WHEN the file size is less than or equal to min_bytes THEN the system SHALL skip the file and create a `.av1skip` sidecar
3. WHEN the file size is greater than min_bytes THEN the system SHALL proceed with further evaluation
4. WHEN a file is skipped due to size THEN the system SHALL write the reason to a `.why.txt` sidecar file

### Requirement 9

**User Story:** As a media library owner, I want the daemon to skip files that are already encoded in AV1, so that processing time is not wasted on files that are already in the target format.

#### Acceptance Criteria

1. WHEN video metadata is available THEN the system SHALL check the video codec name
2. WHEN the video codec is "av1" THEN the system SHALL skip the file and create a `.av1skip` sidecar
3. WHEN the video codec is not "av1" THEN the system SHALL proceed with encoding
4. WHEN a file is skipped due to AV1 codec THEN the system SHALL write the reason to a `.why.txt` sidecar file

### Requirement 10

**User Story:** As a media library owner, I want the daemon to classify video sources as WebLike or DiscLike, so that appropriate encoding safeguards can be applied.

#### Acceptance Criteria

1. WHEN video metadata is available THEN the system SHALL analyze the source characteristics and assign a classification score
2. WHEN the file path contains keywords like "WEB", "WEBRip", "WEBDL", "NF", "AMZN", "DSNP" THEN the system SHALL increase the WebLike score
3. WHEN the video bitrate is below typical streaming thresholds THEN the system SHALL increase the WebLike score
4. WHEN the file path contains keywords like "BluRay", "Remux", "BDMV" THEN the system SHALL increase the DiscLike score
5. WHEN the video bitrate is above typical disc thresholds THEN the system SHALL increase the DiscLike score
6. WHEN the WebLike score exceeds the DiscLike score THEN the system SHALL classify the source as WebLike
7. WHEN the DiscLike score exceeds the WebLike score THEN the system SHALL classify the source as DiscLike
8. WHEN scores are equal THEN the system SHALL classify the source as Unknown

### Requirement 11

**User Story:** As a media library owner, I want the daemon to select appropriate CRF values based on video resolution, so that encoding quality matches the source material characteristics.

#### Acceptance Criteria

1. WHEN video height is 2160 pixels or greater THEN the system SHALL use CRF value 21 for quality-first encoding
2. WHEN video height is 1440 pixels THEN the system SHALL use CRF value 22 for quality-first encoding
3. WHEN video height is 1080 pixels THEN the system SHALL use CRF value 23 for quality-first encoding
4. WHEN video height is less than 1080 pixels THEN the system SHALL use CRF value 24 for quality-first encoding
5. WHEN the original video bitrate is exceptionally low for its resolution THEN the system SHALL increase CRF by 1 within the resolution range

### Requirement 12

**User Story:** As a media library owner, I want the daemon to select appropriate SVT-AV1 preset values based on video resolution, so that encoding speed and quality are balanced for the available hardware.

#### Acceptance Criteria

1. WHEN SVT-AV1 is the selected encoder and video height is 2160 pixels or greater THEN the system SHALL use preset 3
2. WHEN SVT-AV1 is the selected encoder and video height is 1440 pixels THEN the system SHALL use preset 4
3. WHEN SVT-AV1 is the selected encoder and video height is 1080 pixels THEN the system SHALL use preset 4
4. WHEN SVT-AV1 is the selected encoder and video height is less than 1080 pixels THEN the system SHALL use preset 5
5. WHEN the configuration specifies quality_tier as "very_high" THEN the system SHALL decrease the preset value by 1

### Requirement 13

**User Story:** As a media library owner, I want the daemon to construct ffmpeg commands with appropriate stream mapping, so that Russian audio and subtitle tracks are removed while preserving all other streams.

#### Acceptance Criteria

1. WHEN constructing an ffmpeg command THEN the system SHALL include `-map 0` to select all streams initially
2. WHEN constructing an ffmpeg command THEN the system SHALL include `-map -0:v:m:attached_pic` to exclude attached pictures
3. WHEN constructing an ffmpeg command THEN the system SHALL include `-map -0:a:m:language:ru` and `-map -0:a:m:language:rus` to exclude Russian audio
4. WHEN constructing an ffmpeg command THEN the system SHALL include `-map -0:s:m:language:ru` and `-map -0:s:m:language:rus` to exclude Russian subtitles
5. WHEN constructing an ffmpeg command THEN the system SHALL include `-map_chapters 0` and `-map_metadata 0` to preserve chapters and metadata

### Requirement 14

**User Story:** As a media library owner, I want the daemon to apply WebRip-safe ffmpeg flags for web sources, so that timestamp and synchronization issues are avoided.

#### Acceptance Criteria

1. WHEN the source is classified as WebLike THEN the system SHALL include `-fflags +genpts` in the ffmpeg command
2. WHEN the source is classified as WebLike THEN the system SHALL include `-copyts -start_at_zero` in the ffmpeg command
3. WHEN the source is classified as WebLike THEN the system SHALL include `-vsync 0 -avoid_negative_ts make_zero` in the ffmpeg command
4. WHEN the source is not classified as WebLike THEN the system SHALL omit these flags

### Requirement 15

**User Story:** As a media library owner, I want the daemon to apply padding filters when necessary, so that videos with odd dimensions or web sources are compatible with AV1 encoding requirements.

#### Acceptance Criteria

1. WHEN the source is classified as WebLike OR video width is odd OR video height is odd THEN the system SHALL include a pad filter in the ffmpeg command
2. WHEN the pad filter is applied THEN the system SHALL use `-vf "pad=ceil(iw/2)*2:ceil(ih/2)*2,setsar=1"` to ensure even dimensions
3. WHEN the source is not WebLike and dimensions are even THEN the system SHALL omit the pad filter

### Requirement 16

**User Story:** As a media library owner, I want the daemon to encode videos using SVT-AV1 with quality-optimized settings, so that output quality is maximized while maintaining reasonable encoding speed.

#### Acceptance Criteria

1. WHEN SVT-AV1 is the selected encoder THEN the system SHALL include `-c:v libsvtav1` in the ffmpeg command
2. WHEN SVT-AV1 is the selected encoder THEN the system SHALL include `-crf {CRF}` with the resolution-appropriate CRF value
3. WHEN SVT-AV1 is the selected encoder THEN the system SHALL include `-preset {PRESET}` with the resolution-appropriate preset value
4. WHEN SVT-AV1 is the selected encoder THEN the system SHALL include `-threads 0` to allow automatic thread allocation
5. WHEN SVT-AV1 is the selected encoder THEN the system SHALL include `-svtav1-params "lp=0"` for automatic logical processor usage

### Requirement 17

**User Story:** As a media library owner, I want the daemon to encode videos using libaom-av1 with quality-optimized settings when SVT-AV1 is unavailable, so that high-quality output is still achieved.

#### Acceptance Criteria

1. WHEN libaom-av1 is the selected encoder THEN the system SHALL include `-c:v libaom-av1` in the ffmpeg command
2. WHEN libaom-av1 is the selected encoder THEN the system SHALL include `-b:v 0 -crf {CRF}` for constant quality mode
3. WHEN libaom-av1 is the selected encoder and video height is 1080 pixels or less THEN the system SHALL include `-cpu-used 4` or `-cpu-used 5`
4. WHEN libaom-av1 is the selected encoder and video height is greater than 1080 pixels THEN the system SHALL include `-cpu-used 3`
5. WHEN libaom-av1 is the selected encoder THEN the system SHALL include `-row-mt 1` for row-based multithreading
6. WHEN libaom-av1 is the selected encoder and video height is 1080 pixels or less THEN the system SHALL include `-tiles 2x1`
7. WHEN libaom-av1 is the selected encoder and video height is between 1081 and 2160 pixels THEN the system SHALL include `-tiles 2x2`
8. WHEN libaom-av1 is the selected encoder and video height is greater than 2160 pixels THEN the system SHALL include `-tiles 3x2`

### Requirement 18

**User Story:** As a media library owner, I want the daemon to copy audio and subtitle streams without re-encoding, so that quality is preserved and encoding time is minimized.

#### Acceptance Criteria

1. WHEN constructing an ffmpeg command THEN the system SHALL include `-c:a copy` to copy audio streams
2. WHEN constructing an ffmpeg command THEN the system SHALL include `-c:s copy` to copy subtitle streams
3. WHEN constructing an ffmpeg command THEN the system SHALL include `-max_muxing_queue_size 2048` to handle complex multiplexing

### Requirement 19

**User Story:** As a media library owner, I want the daemon to execute encoding jobs asynchronously, so that the system can process multiple files concurrently on the EPYC processor.

#### Acceptance Criteria

1. WHEN a job is ready to execute THEN the system SHALL spawn an asynchronous tokio task
2. WHEN the number of running jobs equals max_concurrent_jobs THEN the system SHALL queue new jobs until a slot becomes available
3. WHEN an encoding job starts THEN the system SHALL update the job status to "running" and record the start timestamp
4. WHEN an encoding job completes THEN the system SHALL update the job status and record the finish timestamp
5. WHEN an encoding job fails THEN the system SHALL update the job status to "failed" and record the error message

### Requirement 20

**User Story:** As a media library owner, I want the daemon to validate encoded output files, so that corrupted or incomplete encodes are detected before replacing the original.

#### Acceptance Criteria

1. WHEN encoding completes THEN the system SHALL execute `ffprobe -v quiet -print_format json -show_format -show_streams` on the output file
2. WHEN ffprobe fails on the output file THEN the system SHALL mark the job as failed and delete the output file
3. WHEN the output file contains zero AV1 video streams THEN the system SHALL mark the job as failed and delete the output file
4. WHEN the output file contains more than one AV1 video stream THEN the system SHALL mark the job as failed and delete the output file
5. WHEN the output file duration differs from the original by more than 2 seconds THEN the system SHALL mark the job as failed and delete the output file
6. WHEN all validation checks pass THEN the system SHALL proceed to the size gate check

### Requirement 21

**User Story:** As a media library owner, I want the daemon to reject encoded files that do not achieve sufficient size reduction, so that disk space is not wasted on ineffective encodes.

#### Acceptance Criteria

1. WHEN output validation passes THEN the system SHALL compare the output file size to the original file size
2. WHEN the output file size is greater than or equal to original size multiplied by max_size_ratio THEN the system SHALL delete the output file
3. WHEN the size gate rejects an output THEN the system SHALL create a `.av1skip` sidecar file
4. WHEN the size gate rejects an output THEN the system SHALL write the reason to a `.why.txt` sidecar file
5. WHEN the size gate rejects an output THEN the system SHALL mark the job as "skipped"
6. WHEN the output file size is less than original size multiplied by max_size_ratio THEN the system SHALL proceed to atomic replacement

### Requirement 22

**User Story:** As a media library owner, I want the daemon to atomically replace original files with encoded outputs, so that the library remains consistent and no data is lost during replacement.

#### Acceptance Criteria

1. WHEN the size gate passes THEN the system SHALL rename the original file to a temporary name with `.orig` suffix
2. WHEN the original file is renamed THEN the system SHALL rename the output file to the original filename
3. WHEN both renames succeed THEN the system SHALL delete the `.orig` file if keep_original is false
4. WHEN both renames succeed and keep_original is true THEN the system SHALL preserve the `.orig` file
5. WHEN any rename operation fails THEN the system SHALL attempt to restore the original state and mark the job as failed

### Requirement 23

**User Story:** As a media library owner, I want the daemon to persist job state as JSON files, so that job history and status can be queried by the TUI and other tools.

#### Acceptance Criteria

1. WHEN a job is created THEN the system SHALL write a JSON file to the job_state_dir with the job metadata
2. WHEN a job status changes THEN the system SHALL update the corresponding JSON file atomically
3. WHEN a job completes THEN the system SHALL write final metadata including original_bytes, new_bytes, and processing duration
4. WHEN a job fails THEN the system SHALL write the failure reason to the JSON file
5. WHEN the job_state_dir does not exist THEN the system SHALL create it at daemon startup

### Requirement 24

**User Story:** As a system administrator, I want to monitor encoding jobs through a terminal UI, so that I can observe progress and system resource usage in real-time.

#### Acceptance Criteria

1. WHEN the TUI starts THEN the system SHALL load all job JSON files from the job_state_dir
2. WHEN the TUI is running THEN the system SHALL refresh job data every 250 milliseconds
3. WHEN the TUI is running THEN the system SHALL display a table of jobs with status, filename, sizes, and compression ratio
4. WHEN the TUI is running THEN the system SHALL display system metrics including CPU usage, memory usage, and GPU usage
5. WHEN the TUI is running THEN the system SHALL display aggregate statistics including total space saved and success rate

### Requirement 25

**User Story:** As a system administrator, I want to navigate and filter jobs in the TUI, so that I can focus on specific job states or find particular files.

#### Acceptance Criteria

1. WHEN the user presses the up arrow or 'k' key THEN the system SHALL move the selection up one row
2. WHEN the user presses the down arrow or 'j' key THEN the system SHALL move the selection down one row
3. WHEN the user presses 'f' THEN the system SHALL cycle through filters: All, Pending, Running, Success, Failed
4. WHEN a filter is active THEN the system SHALL display only jobs matching the filter criteria
5. WHEN the user presses 's' THEN the system SHALL cycle through sort modes: Date, Size, Status, Savings

### Requirement 26

**User Story:** As a system administrator, I want to view detailed information about a selected job, so that I can diagnose issues or understand encoding results.

#### Acceptance Criteria

1. WHEN the user presses Enter on a selected job THEN the system SHALL display a detail view with full job metadata
2. WHEN the detail view is active THEN the system SHALL display the complete file path, resolution, codec, bitrate, and timestamps
3. WHEN the detail view is active THEN the system SHALL display the failure reason if the job failed
4. WHEN the user presses Enter again THEN the system SHALL return to the table view

### Requirement 27

**User Story:** As a system administrator, I want to see real-time progress for running jobs, so that I can estimate completion time and monitor encoding speed.

#### Acceptance Criteria

1. WHEN a job is running THEN the system SHALL monitor the temporary output file size
2. WHEN the temporary output file size increases THEN the system SHALL calculate the bytes per second write rate
3. WHEN the write rate is known THEN the system SHALL estimate the time remaining based on expected output size
4. WHEN a job is running THEN the system SHALL display a progress bar with percentage, speed, and ETA
5. WHEN a job is running THEN the system SHALL detect the current stage: Probing, Transcoding, Verifying, or Replacing

### Requirement 28

**User Story:** As a system administrator, I want the TUI to adapt to different terminal sizes, so that I can use it on various displays and SSH sessions.

#### Acceptance Criteria

1. WHEN the terminal width is 160 columns or greater THEN the system SHALL display all available table columns
2. WHEN the terminal width is between 120 and 159 columns THEN the system SHALL display essential columns only
3. WHEN the terminal width is between 80 and 119 columns THEN the system SHALL display minimal columns
4. WHEN the terminal width is less than 80 columns THEN the system SHALL display critical information only
5. WHEN the terminal height is less than 12 lines THEN the system SHALL display a warning message

### Requirement 29

**User Story:** As a system administrator, I want to quit the TUI gracefully, so that terminal state is restored properly.

#### Acceptance Criteria

1. WHEN the user presses 'q' THEN the system SHALL exit the TUI and restore the terminal to normal mode
2. WHEN the TUI exits THEN the system SHALL not leave the terminal in raw mode or alternate screen
3. WHEN the TUI exits THEN the system SHALL flush all pending output

### Requirement 30

**User Story:** As a developer, I want the application to be packaged for Debian containers, so that it can be deployed consistently across environments.

#### Acceptance Criteria

1. WHEN building for Debian THEN the system SHALL compile with musl or glibc compatible with Debian stable
2. WHEN building for Debian THEN the system SHALL produce a statically linked binary or include all required dynamic libraries
3. WHEN building for Debian THEN the system SHALL include a systemd service file for daemon management
4. WHEN building for Debian THEN the system SHALL include default configuration files in /etc
5. WHEN building for Debian THEN the system SHALL create necessary directories in /var/lib for job state
