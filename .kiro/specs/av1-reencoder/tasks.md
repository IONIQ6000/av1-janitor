# Implementation Plan

- [x] 1. Set up Rust workspace structure and dependencies
  - Create workspace with three crates: daemon, cli-daemon, cli-tui
  - Add all required dependencies to Cargo.toml
  - Set up module structure in daemon crate
  - _Requirements: All_

- [x] 2. Implement configuration module
  - [x] 2.1 Create configuration data structures
    - Define DaemonConfig struct with all fields
    - Define EncoderPreference and QualityTier enums
    - Implement Default trait for sensible defaults
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7_
  
  - [x] 2.2 Write property test for configuration round-trip
    - **Property 3: Configuration loading and application**
    - **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**
  
  - [x] 2.3 Implement TOML configuration loading
    - Implement load_config function with TOML parsing
    - Handle missing files with defaults
    - Handle invalid TOML with error messages
    - _Requirements: 3.1, 3.7_
  
  - [x] 2.4 Write unit tests for configuration edge cases
    - Test missing configuration file
    - Test invalid TOML syntax
    - Test partial configuration with defaults
    - _Requirements: 3.7_

- [x] 3. Implement startup validation module
  - [x] 3.1 Create FFmpeg version checking
    - Execute `ffmpeg -version` command
    - Parse version string to extract major version
    - Return error if version < 8
    - _Requirements: 1.1, 1.2, 1.3, 1.4_
  
  - [x] 3.2 Write property test for FFmpeg version validation
    - **Property 1: FFmpeg version validation**
    - **Validates: Requirements 1.1, 1.2, 1.3**
  
  - [x] 3.3 Create encoder detection
    - Execute `ffmpeg -hide_banner -encoders` command
    - Parse output to detect libsvtav1, libaom-av1, librav1e
    - Return list of available encoders
    - _Requirements: 2.1_
  
  - [x] 3.4 Implement encoder selection logic
    - Select SVT-AV1 if available
    - Fall back to libaom-av1 if SVT not available
    - Fall back to librav1e if neither available
    - Return error if no encoders available
    - _Requirements: 2.2, 2.3, 2.4, 2.5_
  
  - [x] 3.5 Write property test for encoder selection hierarchy
    - **Property 2: Encoder selection hierarchy**
    - **Validates: Requirements 2.2, 2.3, 2.4**

- [x] 4. Implement file scanning module
  - [x] 4.1 Create directory traversal
    - Implement recursive directory scanning
    - Filter by allowed video extensions (.mkv, .mp4, .avi, .mov, .m4v, .ts, .m2ts)
    - Collect file metadata (path, size, modified time)
    - Handle inaccessible directories gracefully
    - _Requirements: 4.1, 4.2, 4.3, 4.4_
  
  - [x] 4.2 Write property test for recursive file discovery
    - **Property 4: Recursive file discovery**
    - **Validates: Requirements 4.1, 4.2, 4.3**
  
  - [x] 4.3 Implement skip marker checking
    - Check for .av1skip file existence
    - Skip files with skip markers
    - _Requirements: 5.1, 5.2, 5.3_
  
  - [x] 4.4 Write property test for skip marker enforcement
    - **Property 5: Skip marker enforcement**
    - **Validates: Requirements 5.1, 5.2, 5.3**

- [x] 5. Implement stable file detection module
  - [x] 5.1 Create stability checker
    - Record initial file size
    - Wait configured duration (10 seconds)
    - Check file size again
    - Return true if sizes match, false otherwise
    - _Requirements: 6.1, 6.2, 6.3, 6.4_
  
  - [x] 5.2 Write property test for stable file detection
    - **Property 6: Stable file detection**
    - **Validates: Requirements 6.3, 6.4**

- [x] 6. Implement FFprobe metadata extraction module
  - [x] 6.1 Create probe execution and JSON parsing
    - Execute ffprobe with JSON output format
    - Parse JSON to extract format and stream information
    - Handle probe failures gracefully
    - _Requirements: 7.1, 7.2, 7.3_
  
  - [x] 6.2 Write property test for FFprobe JSON parsing
    - **Property 7: FFprobe JSON parsing**
    - **Validates: Requirements 7.1, 7.2**
  
  - [x] 6.3 Implement main video stream selection
    - Prefer stream with default disposition
    - Fall back to first video stream
    - Return None if no video streams
    - _Requirements: 7.4, 7.5_
  
  - [x] 6.4 Write property test for main video stream selection
    - **Property 8: Main video stream selection**
    - **Validates: Requirements 7.4**

- [-] 7. Implement source classification module
  - [x] 7.1 Create classification scoring system
    - Implement WebLike keyword detection in paths
    - Implement DiscLike keyword detection in paths
    - Implement bitrate-based scoring
    - Calculate final classification based on scores
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7, 10.8_
  
  - [x] 7.2 Write property test for source classification scoring
    - **Property 11: Source classification scoring**
    - **Validates: Requirements 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7, 10.8**

- [x] 8. Implement gates module
  - [x] 8.1 Create gate evaluation logic
    - Check for skip marker
    - Check for video streams
    - Check file size against min_bytes
    - Check if codec is already AV1
    - Return Pass or Skip with reason
    - _Requirements: 8.1, 8.2, 8.3, 9.1, 9.2, 9.3_
  
  - [x] 8.2 Write property test for size threshold enforcement
    - **Property 9: Size threshold enforcement**
    - **Validates: Requirements 8.1, 8.2, 8.3, 8.4**
  
  - [x] 8.3 Write property test for AV1 codec detection and skip
    - **Property 10: AV1 codec detection and skip**
    - **Validates: Requirements 9.1, 9.2, 9.3, 9.4**

- [x] 9. Implement encoder parameter selection
  - [x] 9.1 Create CRF selection logic
    - Implement resolution-based CRF ladder
    - Implement bitrate-based CRF adjustment
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_
  
  - [x] 9.2 Write property test for CRF selection by resolution
    - **Property 12: CRF selection by resolution**
    - **Validates: Requirements 11.1, 11.2, 11.3, 11.4, 11.5**
  
  - [x] 9.3 Create SVT-AV1 preset selection logic
    - Implement resolution-based preset selection
    - Implement quality tier adjustment
    - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5_
  
  - [x] 9.4 Write property test for SVT-AV1 preset selection
    - **Property 13: SVT-AV1 preset selection**
    - **Validates: Requirements 12.1, 12.2, 12.3, 12.4, 12.5**
  
  - [x] 9.5 Create libaom-av1 parameter selection
    - Implement cpu-used selection based on resolution
    - Implement tile configuration based on resolution
    - _Requirements: 17.3, 17.4, 17.6, 17.7, 17.8_

- [x] 10. Implement FFmpeg command construction
  - [x] 10.1 Create common command components
    - Implement stream mapping flags
    - Implement Russian audio/subtitle exclusion
    - Implement chapter and metadata preservation
    - _Requirements: 13.1, 13.2, 13.3, 13.4, 13.5_
  
  - [x] 10.2 Write property test for stream mapping command construction
    - **Property 14: Stream mapping command construction**
    - **Validates: Requirements 13.1, 13.2, 13.3, 13.4, 13.5**
  
  - [x] 10.3 Implement WebLike-specific flags
    - Add genpts, copyts, start_at_zero flags for WebLike sources
    - Omit flags for non-WebLike sources
    - _Requirements: 14.1, 14.2, 14.3, 14.4_
  
  - [x] 10.4 Write property test for WebLike flag inclusion
    - **Property 15: WebLike flag inclusion**
    - **Validates: Requirements 14.1, 14.2, 14.3, 14.4**
  
  - [x] 10.5 Implement pad filter logic
    - Add pad filter for WebLike sources or odd dimensions
    - Omit filter when not needed
    - _Requirements: 15.1, 15.2, 15.3_
  
  - [x] 10.6 Write property test for pad filter application
    - **Property 16: Pad filter application**
    - **Validates: Requirements 15.1, 15.2, 15.3**
  
  - [x] 10.7 Create SVT-AV1 command builder
    - Construct complete SVT-AV1 ffmpeg command
    - Include all required parameters
    - _Requirements: 16.1, 16.2, 16.3, 16.4, 16.5_
  
  - [x] 10.8 Write property test for SVT-AV1 command parameters
    - **Property 17: SVT-AV1 command parameters**
    - **Validates: Requirements 16.1, 16.2, 16.3, 16.4, 16.5**
  
  - [x] 10.9 Create libaom-av1 command builder
    - Construct complete libaom-av1 ffmpeg command
    - Include all required parameters
    - _Requirements: 17.1, 17.2, 17.3, 17.4, 17.5, 17.6, 17.7, 17.8_
  
  - [x] 10.10 Write property test for libaom-av1 command parameters
    - **Property 18: libaom-av1 command parameters**
    - **Validates: Requirements 17.1, 17.2, 17.3, 17.4, 17.5, 17.6, 17.7, 17.8**
  
  - [x] 10.11 Create librav1e command builder
    - Construct complete librav1e ffmpeg command
    - Use fallback parameters
    - _Requirements: 18.1, 18.2, 18.3_
  
  - [x] 10.12 Write property test for audio and subtitle stream copying
    - **Property 19: Audio and subtitle stream copying**
    - **Validates: Requirements 18.1, 18.2, 18.3**

- [x] 11. Implement job management module
  - [x] 11.1 Create Job data structure
    - Define Job struct with all fields
    - Define JobStatus enum
    - Implement job creation function
    - _Requirements: 23.1_
  
  - [x] 11.2 Implement job persistence
    - Implement save_job function with atomic writes
    - Implement load_all_jobs function
    - Handle JSON serialization/deserialization
    - _Requirements: 23.1, 23.2, 23.3, 23.4_
  
  - [x] 11.3 Write property test for job persistence
    - **Property 25: Job persistence**
    - **Validates: Requirements 23.1, 23.2, 23.3, 23.4**
  
  - [x] 11.4 Implement job status transitions
    - Update status with timestamp recording
    - Validate state machine transitions
    - _Requirements: 19.3, 19.4, 19.5_
  
  - [x] 11.5 Write property test for job status transitions
    - **Property 21: Job status transitions**
    - **Validates: Requirements 19.3, 19.4, 19.5**

- [x] 12. Implement encoding execution module
  - [x] 12.1 Create FFmpeg process spawning
    - Spawn ffmpeg as async child process
    - Capture stdout/stderr for logging
    - Monitor process completion
    - _Requirements: 19.1_
  
  - [x] 12.2 Implement concurrent job limiting
    - Create job queue with max_concurrent_jobs limit
    - Spawn jobs when slots available
    - Track running jobs
    - _Requirements: 19.2_
  
  - [x] 12.3 Write property test for concurrent job limiting
    - **Property 20: Concurrent job limiting**
    - **Validates: Requirements 19.2**

- [x] 13. Implement output validation module
  - [x] 13.1 Create validation logic
    - Execute ffprobe on output file
    - Check for exactly one AV1 video stream
    - Check duration matches original within epsilon
    - _Requirements: 20.1, 20.2, 20.3, 20.4, 20.5, 20.6_
  
  - [x] 13.2 Write property test for output validation
    - **Property 22: Output validation**
    - **Validates: Requirements 20.1, 20.6**

- [x] 14. Implement size gate module
  - [x] 14.1 Create size comparison logic
    - Compare output size to original * max_size_ratio
    - Calculate compression ratio and savings
    - Return Pass or Fail result
    - _Requirements: 21.1, 21.2, 21.6_
  
  - [x] 14.2 Write property test for size gate enforcement
    - **Property 23: Size gate enforcement**
    - **Validates: Requirements 21.1, 21.2, 21.3, 21.4, 21.5, 21.6**

- [x] 15. Implement sidecar file management module
  - [x] 15.1 Create sidecar file functions
    - Implement create_skip_marker function
    - Implement write_why_file function
    - Implement has_skip_marker function
    - _Requirements: 5.1, 8.4, 9.4, 21.3, 21.4_

- [x] 16. Implement atomic file replacement module
  - [x] 16.1 Create replacement logic
    - Rename original to .orig
    - Rename output to original name
    - Handle keep_original flag
    - Implement rollback on failure
    - _Requirements: 22.1, 22.2, 22.3, 22.4, 22.5_
  
  - [x] 16.2 Write property test for atomic file replacement
    - **Property 24: Atomic file replacement**
    - **Validates: Requirements 22.1, 22.2, 22.3, 22.4**

- [x] 17. Implement daemon main loop
  - [x] 17.1 Create scan loop
    - Implement periodic scanning with scan_interval_secs
    - Integrate all modules: scan → probe → classify → gates → encode → validate → replace
    - Handle errors gracefully at each stage
    - _Requirements: 4.5, All workflow requirements_
  
  - [x] 17.2 Implement daemon CLI
    - Parse command line arguments for config path
    - Initialize configuration
    - Run startup validation
    - Start main loop
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 2.5_

- [x] 18. Integrate existing TUI package
  - [x] 18.1 Copy TUI source files
    - Copy tui-package/src files to cli-tui crate
    - Update module paths and imports
    - _Requirements: 24.1_
  
  - [x] 18.2 Adapt TUI to use daemon Job structure
    - Ensure Job struct matches between daemon and TUI
    - Update any TUI-specific fields if needed
    - _Requirements: 24.1, 24.3, 24.4, 24.5_
  
  - [x] 18.3 Write property test for TUI job loading
    - **Property 26: TUI job loading**
    - **Validates: Requirements 24.1**
  
  - [x] 18.4 Write property test for statistics calculation
    - **Property 27: Statistics calculation**
    - **Validates: Requirements 24.5**
  
  - [x] 18.5 Write property test for job filtering
    - **Property 28: Job filtering**
    - **Validates: Requirements 25.3, 25.4**
  
  - [x] 18.6 Write property test for sort mode cycling
    - **Property 29: Sort mode cycling**
    - **Validates: Requirements 25.5**
  
  - [x] 18.7 Write property test for progress rate calculation
    - **Property 30: Progress rate calculation**
    - **Validates: Requirements 27.1, 27.2**
  
  - [x] 18.8 Write property test for ETA estimation
    - **Property 31: ETA estimation**
    - **Validates: Requirements 27.3**
  
  - [x] 18.9 Write property test for stage detection
    - **Property 32: Stage detection**
    - **Validates: Requirements 27.5**
  
  - [x] 18.10 Write property test for responsive column layout
    - **Property 33: Responsive column layout**
    - **Validates: Requirements 28.1, 28.2, 28.3, 28.4, 28.5**

- [x] 19. Create Debian container packaging
  - [x] 19.1 Create Dockerfile
    - Use debian:bookworm-slim base image
    - Install ffmpeg system dependency
    - Copy compiled binaries
    - Set up directories and permissions
    - _Requirements: 30.1, 30.2_
  
  - [x] 19.2 Create systemd service file
    - Define service configuration
    - Set up restart policies
    - Configure user and permissions
    - _Requirements: 30.3_
  
  - [x] 19.3 Create default configuration file
    - Write default config.toml
    - Document all configuration options
    - _Requirements: 30.4_
  
  - [x] 19.4 Create installation script
    - Create necessary directories
    - Set up permissions
    - Install service file
    - _Requirements: 30.5_

- [x] 20. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 21. Create documentation
  - [x] 21.1 Write README.md
    - Document installation process
    - Document configuration options
    - Document usage examples
    - Include troubleshooting section
  
  - [x] 21.2 Write deployment guide
    - Document Debian container setup
    - Document systemd service management
    - Document monitoring with TUI
  
  - [x] 21.3 Write performance tuning guide
    - Document EPYC-specific recommendations
    - Document quality vs speed tradeoffs
    - Document storage considerations

- [ ] 22. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.
