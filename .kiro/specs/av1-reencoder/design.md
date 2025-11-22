# Design Document

## Overview

This document describes the design of a Rust-based AV1 video re-encoding daemon with a terminal user interface (TUI). The system is designed for quality-first software AV1 encoding on a 32-core AMD EPYC processor running in a Debian container.

The architecture follows a modular design with clear separation between:
- **Daemon core**: Scanning, classification, encoding orchestration
- **Encoding engine**: FFmpeg command construction and execution
- **Job management**: State persistence and lifecycle tracking
- **TUI**: Real-time monitoring and user interaction

The system prioritizes video quality through conservative CRF values, slower encoder presets, and intelligent source classification. It implements safety mechanisms including stable-file detection, size gates, and atomic file replacement to ensure library integrity.

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         TUI (av1top)                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Job Monitor  │  │ System Stats │  │ User Input   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ Reads JSON
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Job State Store                         │
│                  (JSON files on disk)                        │
└─────────────────────────────────────────────────────────────┘
                              ▲
                              │ Writes JSON
                              │
┌─────────────────────────────────────────────────────────────┐
│                      Daemon (av1d)                           │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Scan & Discovery Loop                   │   │
│  │  • Recursive directory traversal                     │   │
│  │  • Stable file detection                             │   │
│  │  • Skip marker checking                              │   │
│  └──────────────────────────────────────────────────────┘   │
│                              │                               │
│                              ▼                               │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Probe & Classification                  │   │
│  │  • FFprobe metadata extraction                       │   │
│  │  • Source type classification (Web/Disc)             │   │
│  │  • Gate evaluation (size, codec, etc.)               │   │
│  └──────────────────────────────────────────────────────┘   │
│                              │                               │
│                              ▼                               │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Encoding Orchestration                  │   │
│  │  • Encoder selection (SVT/AOM/rav1e)                 │   │
│  │  • CRF/preset calculation                            │   │
│  │  • FFmpeg command construction                       │   │
│  │  • Concurrent job management                         │   │
│  └──────────────────────────────────────────────────────┘   │
│                              │                               │
│                              ▼                               │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Validation & Replacement                │   │
│  │  • Output validation (FFprobe)                       │   │
│  │  • Size gate enforcement                             │   │
│  │  • Atomic file replacement                           │   │
│  │  • Sidecar file management                           │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ Executes
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    FFmpeg / FFprobe                          │
│                  (System binaries ≥ 8.0)                     │
└─────────────────────────────────────────────────────────────┘
```

### Process Model

The system consists of two independent processes:

1. **Daemon Process (`av1d`)**: Long-running background service that scans directories, creates jobs, executes encoding, and manages state
2. **TUI Process (`av1top`)**: Interactive terminal application that reads job state and displays real-time information

These processes communicate through the filesystem via JSON job files, enabling loose coupling and independent operation.

### Concurrency Model

The daemon uses Tokio for asynchronous I/O and task management:

- **Main scan loop**: Single-threaded sequential directory traversal
- **Job execution**: Configurable concurrent job pool (default: 1 job at a time for quality)
- **FFmpeg processes**: Spawned as child processes with `tokio::process::Command`
- **File I/O**: Asynchronous file operations using `tokio::fs`

The TUI uses:

- **Main event loop**: Handles keyboard input and rendering at 250ms intervals
- **Job loading**: Synchronous file I/O on refresh
- **System metrics**: Polled from `sysinfo` crate

## Components and Interfaces

### Daemon Components

#### 1. Configuration Module (`config.rs`)

**Responsibility**: Load and validate configuration from TOML files

**Key Types**:
```rust
pub struct DaemonConfig {
    pub library_roots: Vec<PathBuf>,
    pub min_bytes: u64,
    pub max_size_ratio: f64,
    pub scan_interval_secs: u64,
    pub job_state_dir: PathBuf,
    pub temp_output_dir: PathBuf,
    pub max_concurrent_jobs: usize,
    pub prefer_encoder: EncoderPreference,
    pub quality_tier: QualityTier,
    pub keep_original: bool,
    pub write_why_sidecars: bool,
}

pub enum EncoderPreference {
    Svt,
    Aom,
    Rav1e,
}

pub enum QualityTier {
    High,
    VeryHigh,
}
```

**Interface**:
- `load_config(path: Option<&Path>) -> Result<DaemonConfig>`
- `validate_config(config: &DaemonConfig) -> Result<()>`

#### 2. Startup Validation Module (`startup.rs`)

**Responsibility**: Verify system prerequisites before daemon operation

**Key Functions**:
- `check_ffmpeg_version() -> Result<(u32, u32, u32)>`: Parse and validate ffmpeg version
- `detect_available_encoders() -> Result<Vec<AvailableEncoder>>`: Query ffmpeg for AV1 encoders
- `select_encoder(available: &[AvailableEncoder], preference: EncoderPreference) -> Result<SelectedEncoder>`: Choose encoder based on availability and preference

**Key Types**:
```rust
pub enum AvailableEncoder {
    SvtAv1,
    LibaomAv1,
    Librav1e,
}

pub struct SelectedEncoder {
    pub encoder: AvailableEncoder,
    pub codec_name: String, // "libsvtav1", "libaom-av1", "librav1e"
}
```

#### 3. Scanner Module (`scan.rs`)

**Responsibility**: Recursively discover video files in library directories

**Key Functions**:
- `scan_libraries(roots: &[PathBuf]) -> Result<Vec<CandidateFile>>`: Traverse directories and collect video files
- `is_video_file(path: &Path) -> bool`: Check file extension against allowed list

**Key Types**:
```rust
pub struct CandidateFile {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_time: SystemTime,
}
```

**Allowed Extensions**: `.mkv`, `.mp4`, `.avi`, `.mov`, `.m4v`, `.ts`, `.m2ts`

#### 4. Stable File Module (`stable.rs`)

**Responsibility**: Detect files that are actively being written

**Key Functions**:
- `check_stability(file: &CandidateFile, duration: Duration) -> Result<bool>`: Wait and compare file sizes

**Algorithm**:
1. Record initial file size
2. Sleep for configured duration (default: 10 seconds)
3. Check file size again
4. Return `true` if sizes match, `false` otherwise

#### 5. Probe Module (`probe.rs`)

**Responsibility**: Extract video metadata using ffprobe

**Key Functions**:
- `probe_file(path: &Path) -> Result<ProbeResult>`: Execute ffprobe and parse JSON output
- `select_main_video_stream(streams: &[VideoStream]) -> Option<&VideoStream>`: Choose primary video stream

**Key Types**:
```rust
pub struct ProbeResult {
    pub format: FormatInfo,
    pub video_streams: Vec<VideoStream>,
    pub audio_streams: Vec<AudioStream>,
    pub subtitle_streams: Vec<SubtitleStream>,
}

pub struct VideoStream {
    pub index: usize,
    pub codec_name: String,
    pub width: i32,
    pub height: i32,
    pub bitrate: Option<u64>,
    pub frame_rate: Option<String>,
    pub pix_fmt: Option<String>,
    pub bit_depth: Option<u8>,
    pub is_default: bool,
}

pub struct FormatInfo {
    pub duration: Option<f64>,
    pub size: u64,
    pub bitrate: Option<u64>,
}
```

**Stream Selection Logic**:
1. Prefer stream with `disposition.default = 1`
2. Fall back to first video stream
3. Return `None` if no video streams exist

#### 6. Classification Module (`classify.rs`)

**Responsibility**: Classify video sources as WebLike, DiscLike, or Unknown

**Key Functions**:
- `classify_source(path: &Path, probe: &ProbeResult) -> SourceClassification`: Score and classify source

**Key Types**:
```rust
pub enum SourceType {
    WebLike,
    DiscLike,
    Unknown,
}

pub struct SourceClassification {
    pub source_type: SourceType,
    pub web_score: i32,
    pub disc_score: i32,
    pub reasons: Vec<String>,
}
```

**Scoring Rules**:

*WebLike indicators* (+score):
- Path contains: `WEB`, `WEBRip`, `WEBDL`, `WEB-DL`, `NF`, `AMZN`, `DSNP`, `HULU`, `ATVP` (+10 each)
- Bitrate < 5 Mbps for 1080p (+5)
- Bitrate < 10 Mbps for 2160p (+5)
- Codec is VP9 (+5)

*DiscLike indicators* (+score):
- Path contains: `BluRay`, `Blu-ray`, `Remux`, `BDMV`, `UHD` (+10 each)
- Bitrate > 15 Mbps for 1080p (+5)
- Bitrate > 40 Mbps for 2160p (+5)
- File size > 20 GB (+5)

**Classification Decision**:
- If `web_score > disc_score`: WebLike
- If `disc_score > web_score`: DiscLike
- If `web_score == disc_score`: Unknown

#### 7. Gates Module (`gates.rs`)

**Responsibility**: Evaluate skip conditions before encoding

**Key Functions**:
- `check_gates(file: &CandidateFile, probe: &ProbeResult, config: &DaemonConfig) -> GateResult`: Evaluate all gates

**Key Types**:
```rust
pub enum GateResult {
    Pass,
    Skip(SkipReason),
}

pub enum SkipReason {
    NoVideo,
    TooSmall,
    AlreadyAv1,
    HasSkipMarker,
}
```

**Gate Evaluation Order**:
1. Skip marker exists (`.av1skip` file)
2. No video streams
3. File size ≤ `min_bytes`
4. Video codec is already AV1

#### 8. Encoder Module (`encode/`)

**Responsibility**: Construct and execute FFmpeg commands

**Submodules**:
- `encode/svt.rs`: SVT-AV1 command builder
- `encode/aom.rs`: libaom-av1 command builder
- `encode/rav1e.rs`: librav1e command builder
- `encode/common.rs`: Shared command components

**Key Functions**:
- `build_command(job: &Job, encoder: &SelectedEncoder, config: &DaemonConfig) -> Vec<String>`: Construct FFmpeg arguments
- `execute_encode(job: &mut Job, command: Vec<String>) -> Result<PathBuf>`: Spawn FFmpeg process and monitor

**CRF Selection Logic**:
```rust
fn select_crf(height: i32, bitrate: Option<u64>) -> u8 {
    let base_crf = match height {
        h if h >= 2160 => 21,
        h if h >= 1440 => 22,
        h if h >= 1080 => 23,
        _ => 24,
    };
    
    // Increase CRF by 1 if bitrate is exceptionally low
    if let Some(br) = bitrate {
        let threshold = match height {
            h if h >= 2160 => 20_000_000, // 20 Mbps
            h if h >= 1440 => 10_000_000, // 10 Mbps
            h if h >= 1080 => 5_000_000,  // 5 Mbps
            _ => 2_000_000,               // 2 Mbps
        };
        if br < threshold {
            return base_crf + 1;
        }
    }
    
    base_crf
}
```

**Preset Selection Logic (SVT-AV1)**:
```rust
fn select_preset(height: i32, quality_tier: QualityTier) -> u8 {
    let base_preset = match height {
        h if h >= 2160 => 3,
        h if h >= 1440 => 4,
        h if h >= 1080 => 4,
        _ => 5,
    };
    
    match quality_tier {
        QualityTier::High => base_preset,
        QualityTier::VeryHigh => base_preset.saturating_sub(1),
    }
}
```

**FFmpeg Command Template (SVT-AV1)**:
```bash
ffmpeg -hide_banner -y \
  {WEBSAFE_INPUT_FLAGS} \
  -i "{INPUT_PATH}" \
  -map 0 \
  -map -0:v:m:attached_pic \
  -map 0:v:{VIDEO_STREAM_INDEX} \
  -map 0:a? -map -0:a:m:language:ru -map -0:a:m:language:rus \
  -map 0:s? -map -0:s:m:language:ru -map -0:s:m:language:rus \
  -map_chapters 0 -map_metadata 0 \
  {PAD_FILTER} \
  -c:v libsvtav1 -crf {CRF} -preset {PRESET} -threads 0 \
  -svtav1-params "lp=0" \
  -c:a copy -c:s copy \
  -max_muxing_queue_size 2048 \
  {WEBSAFE_OUTPUT_FLAGS} \
  "{OUTPUT_PATH}"
```

**WebSafe Input Flags** (when `source_type == WebLike`):
```bash
-fflags +genpts -copyts -start_at_zero -vsync 0 -avoid_negative_ts make_zero
```

**Pad Filter** (when `source_type == WebLike` OR width/height is odd):
```bash
-vf "pad=ceil(iw/2)*2:ceil(ih/2)*2,setsar=1"
```

**Tile Configuration (libaom-av1)**:
```rust
fn select_tiles(height: i32) -> &'static str {
    match height {
        h if h > 2160 => "3x2",
        h if h > 1080 => "2x2",
        _ => "2x1",
    }
}
```

#### 9. Validation Module (`validate.rs`)

**Responsibility**: Verify encoded output meets quality standards

**Key Functions**:
- `validate_output(output_path: &Path, original_probe: &ProbeResult) -> Result<ValidationResult>`: Check output file

**Key Types**:
```rust
pub enum ValidationResult {
    Valid(ProbeResult),
    Invalid(ValidationError),
}

pub enum ValidationError {
    ProbeFailure(String),
    NoAv1Stream,
    MultipleAv1Streams,
    DurationMismatch { expected: f64, actual: f64 },
}
```

**Validation Checks**:
1. FFprobe can read the file
2. Exactly one AV1 video stream exists
3. Duration matches original within 2 seconds

#### 10. Size Gate Module (`size_gate.rs`)

**Responsibility**: Enforce size reduction requirements

**Key Functions**:
- `check_size_gate(original_bytes: u64, new_bytes: u64, max_ratio: f64) -> SizeGateResult`: Compare sizes

**Key Types**:
```rust
pub enum SizeGateResult {
    Pass { savings_bytes: u64, compression_ratio: f64 },
    Fail { new_bytes: u64, threshold_bytes: u64 },
}
```

**Logic**:
```rust
let threshold = (original_bytes as f64 * max_ratio) as u64;
if new_bytes >= threshold {
    SizeGateResult::Fail { new_bytes, threshold_bytes: threshold }
} else {
    let savings = original_bytes - new_bytes;
    let ratio = (new_bytes as f64) / (original_bytes as f64);
    SizeGateResult::Pass { savings_bytes: savings, compression_ratio: ratio }
}
```

#### 11. Replacement Module (`replace.rs`)

**Responsibility**: Atomically replace original files with encoded outputs

**Key Functions**:
- `atomic_replace(original: &Path, new: &Path, keep_original: bool) -> Result<()>`: Perform atomic replacement

**Algorithm**:
1. Generate temporary name: `{original}.orig.{timestamp}`
2. Rename original → temp name
3. Rename new → original name
4. If `keep_original == false`: delete temp file
5. On any error: attempt to restore original from temp

#### 12. Sidecar Module (`sidecars.rs`)

**Responsibility**: Manage `.av1skip` and `.why.txt` files

**Key Functions**:
- `create_skip_marker(video_path: &Path) -> Result<()>`: Create empty `.av1skip` file
- `write_why_file(video_path: &Path, reason: &str) -> Result<()>`: Write reason to `.why.txt`
- `has_skip_marker(video_path: &Path) -> bool`: Check for `.av1skip` existence

#### 13. Job Management Module (`jobs.rs`)

**Responsibility**: Manage job lifecycle and state persistence

**Key Functions**:
- `create_job(file: CandidateFile, probe: ProbeResult, classification: SourceClassification) -> Job`: Initialize new job
- `save_job(job: &Job, state_dir: &Path) -> Result<()>`: Persist job to JSON
- `load_all_jobs(state_dir: &Path) -> Result<Vec<Job>>`: Load all jobs from directory
- `update_job_status(job: &mut Job, status: JobStatus) -> Result<()>`: Update status and save

**Job State Machine**:
```
Pending → Running → Success
                 ↘ Failed
                 ↘ Skipped
```

### TUI Components

The TUI is provided as a pre-built package and integrates with the daemon through the shared `Job` data model.

#### Key TUI Modules

1. **Main Loop** (`main.rs`): Event handling, rendering, keyboard input
2. **Job Display**: Table rendering with responsive column layout
3. **System Metrics**: CPU, memory, GPU usage display
4. **Statistics**: Aggregate metrics calculation and caching
5. **Progress Tracking**: Real-time encoding progress estimation

The TUI reads job JSON files from `job_state_dir` and displays them in real-time. It does not modify job state.

## Data Models

### Job Structure

The `Job` struct is the central data model shared between daemon and TUI:

```rust
pub struct Job {
    // Identity
    pub id: String,
    pub source_path: PathBuf,
    pub output_path: Option<PathBuf>,
    
    // Timestamps
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    
    // Status
    pub status: JobStatus,
    pub reason: Option<String>,
    
    // Size metrics
    pub original_bytes: Option<u64>,
    pub new_bytes: Option<u64>,
    
    // Source classification
    pub is_web_like: bool,
    
    // Video metadata
    pub video_codec: Option<String>,
    pub video_bitrate: Option<u64>,
    pub video_width: Option<i32>,
    pub video_height: Option<i32>,
    pub video_frame_rate: Option<String>,
    
    // Encoding parameters
    pub crf_used: Option<u8>,
    pub preset_used: Option<u8>,
    pub encoder_used: Option<String>,
    
    // Additional metadata
    pub source_bit_depth: Option<u8>,
    pub source_pix_fmt: Option<String>,
    pub is_hdr: Option<bool>,
}

pub enum JobStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
}
```

### Configuration Structure

```rust
pub struct DaemonConfig {
    // Scanning
    pub library_roots: Vec<PathBuf>,
    pub scan_interval_secs: u64,
    
    // Filtering
    pub min_bytes: u64,
    pub max_size_ratio: f64,
    
    // Paths
    pub job_state_dir: PathBuf,
    pub temp_output_dir: PathBuf,
    
    // Encoding
    pub max_concurrent_jobs: usize,
    pub prefer_encoder: EncoderPreference,
    pub quality_tier: QualityTier,
    
    // Behavior
    pub keep_original: bool,
    pub write_why_sidecars: bool,
}
```

### FFprobe JSON Schema

The system parses FFprobe JSON output with this structure:

```json
{
  "format": {
    "duration": "7200.000000",
    "size": "5000000000",
    "bit_rate": "5555555"
  },
  "streams": [
    {
      "index": 0,
      "codec_name": "hevc",
      "codec_type": "video",
      "width": 1920,
      "height": 1080,
      "r_frame_rate": "24/1",
      "pix_fmt": "yuv420p",
      "bits_per_raw_sample": "8",
      "disposition": {
        "default": 1
      }
    }
  ]
}
```

## Corre
ctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

After reviewing the acceptance criteria, many properties can be consolidated as they test similar behaviors across different parameter ranges. The following properties provide comprehensive coverage while avoiding redundancy:

### Property 1: FFmpeg version validation
*For any* ffmpeg version string output, parsing should correctly extract the major version number and reject versions below 8
**Validates: Requirements 1.1, 1.2, 1.3**

### Property 2: Encoder selection hierarchy
*For any* set of available encoders, the system should select SVT-AV1 if available, otherwise libaom-av1 if available, otherwise librav1e if available
**Validates: Requirements 2.2, 2.3, 2.4**

### Property 3: Configuration loading and application
*For any* valid TOML configuration file, all specified values should be correctly loaded and applied to daemon behavior
**Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6**

### Property 4: Recursive file discovery
*For any* directory structure, scanning should discover all files with allowed video extensions in all subdirectories
**Validates: Requirements 4.1, 4.2, 4.3**

### Property 5: Skip marker enforcement
*For any* file with a `.av1skip` sidecar, the system should skip processing and not create a job
**Validates: Requirements 5.1, 5.2, 5.3**

### Property 6: Stable file detection
*For any* file whose size changes between two measurements, the system should mark it as unstable and skip it
**Validates: Requirements 6.3, 6.4**

### Property 7: FFprobe JSON parsing
*For any* valid ffprobe JSON output, the system should correctly extract all video stream metadata fields
**Validates: Requirements 7.1, 7.2**

### Property 8: Main video stream selection
*For any* set of video streams, the system should select the stream with default disposition if present, otherwise the first stream
**Validates: Requirements 7.4**

### Property 9: Size threshold enforcement
*For any* file size and min_bytes threshold, files at or below the threshold should be skipped with appropriate sidecar files
**Validates: Requirements 8.1, 8.2, 8.3, 8.4**

### Property 10: AV1 codec detection and skip
*For any* video with codec name "av1", the system should skip processing and create appropriate sidecar files
**Validates: Requirements 9.1, 9.2, 9.3, 9.4**

### Property 11: Source classification scoring
*For any* file path and video metadata, the classification system should correctly score WebLike and DiscLike indicators and assign the appropriate classification
**Validates: Requirements 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7, 10.8**

### Property 12: CRF selection by resolution
*For any* video height, the system should select the appropriate CRF value according to the quality-first ladder (21 for ≥2160p, 22 for 1440p, 23 for 1080p, 24 for <1080p), with adjustment for low bitrate
**Validates: Requirements 11.1, 11.2, 11.3, 11.4, 11.5**

### Property 13: SVT-AV1 preset selection
*For any* video height and quality tier, the system should select the appropriate SVT-AV1 preset value with quality tier adjustment
**Validates: Requirements 12.1, 12.2, 12.3, 12.4, 12.5**

### Property 14: Stream mapping command construction
*For any* ffmpeg command, it should include all required stream mapping flags to exclude Russian audio/subtitles and preserve chapters/metadata
**Validates: Requirements 13.1, 13.2, 13.3, 13.4, 13.5**

### Property 15: WebLike flag inclusion
*For any* source classified as WebLike, the ffmpeg command should include all WebRip-safe flags; for non-WebLike sources, these flags should be omitted
**Validates: Requirements 14.1, 14.2, 14.3, 14.4**

### Property 16: Pad filter application
*For any* source that is WebLike OR has odd width OR has odd height, the ffmpeg command should include the pad filter; otherwise it should be omitted
**Validates: Requirements 15.1, 15.2, 15.3**

### Property 17: SVT-AV1 command parameters
*For any* job using SVT-AV1 encoder, the ffmpeg command should include all required SVT-AV1 parameters with correct CRF and preset values
**Validates: Requirements 16.1, 16.2, 16.3, 16.4, 16.5**

### Property 18: libaom-av1 command parameters
*For any* job using libaom-av1 encoder, the ffmpeg command should include all required libaom-av1 parameters with correct CRF, cpu-used, and tile configuration based on resolution
**Validates: Requirements 17.1, 17.2, 17.3, 17.4, 17.5, 17.6, 17.7, 17.8**

### Property 19: Audio and subtitle stream copying
*For any* ffmpeg command, it should include parameters to copy audio and subtitle streams without re-encoding
**Validates: Requirements 18.1, 18.2, 18.3**

### Property 20: Concurrent job limiting
*For any* number of pending jobs, the system should never run more than max_concurrent_jobs simultaneously
**Validates: Requirements 19.2**

### Property 21: Job status transitions
*For any* job, status transitions should follow the valid state machine (Pending → Running → Success/Failed/Skipped) with appropriate timestamp updates
**Validates: Requirements 19.3, 19.4, 19.5**

### Property 22: Output validation
*For any* encoded output file, validation should verify ffprobe can read it and proceed to size gate only when all checks pass
**Validates: Requirements 20.1, 20.6**

### Property 23: Size gate enforcement
*For any* original and output file sizes, the size gate should reject outputs ≥ (original × max_size_ratio) and create appropriate sidecar files
**Validates: Requirements 21.1, 21.2, 21.3, 21.4, 21.5, 21.6**

### Property 24: Atomic file replacement
*For any* successful encoding, the replacement process should rename original to .orig, rename output to original name, and handle keep_original flag correctly
**Validates: Requirements 22.1, 22.2, 22.3, 22.4**

### Property 25: Job persistence
*For any* job state change, the corresponding JSON file should be updated atomically with all current metadata
**Validates: Requirements 23.1, 23.2, 23.3, 23.4**

### Property 26: TUI job loading
*For any* job_state_dir containing JSON files, the TUI should successfully load all valid job files
**Validates: Requirements 24.1**

### Property 27: Statistics calculation
*For any* set of jobs, aggregate statistics (total space saved, success rate) should be calculated correctly
**Validates: Requirements 24.5**

### Property 28: Job filtering
*For any* active filter and set of jobs, only jobs matching the filter criteria should be included in the filtered result
**Validates: Requirements 25.3, 25.4**

### Property 29: Sort mode cycling
*For any* current sort mode, cycling should progress through the sequence: Date → Size → Status → Savings → Date
**Validates: Requirements 25.5**

### Property 30: Progress rate calculation
*For any* two file size measurements over time, the bytes per second rate should be calculated correctly
**Validates: Requirements 27.1, 27.2**

### Property 31: ETA estimation
*For any* known write rate and expected output size, the estimated time remaining should be calculated correctly
**Validates: Requirements 27.3**

### Property 32: Stage detection
*For any* job state, the current processing stage should be correctly identified based on job status and file existence
**Validates: Requirements 27.5**

### Property 33: Responsive column layout
*For any* terminal width, the system should display the appropriate set of columns according to the responsive layout rules
**Validates: Requirements 28.1, 28.2, 28.3, 28.4, 28.5**

## Error Handling

### Daemon Error Handling

**Startup Errors** (fail-fast):
- FFmpeg not found or version < 8.0: Abort with clear error message
- No AV1 encoders available: Abort with clear error message
- Configuration file invalid: Abort with parse error details
- Job state directory not writable: Abort with permission error

**Runtime Errors** (graceful degradation):
- Directory scan failure: Log warning, continue with other directories
- FFprobe failure on file: Skip file, create `.av1skip` and `.why.txt`
- FFmpeg encoding failure: Mark job as failed, preserve original file
- Output validation failure: Delete output, mark job as failed
- File system errors during replacement: Attempt rollback, mark job as failed

**Error Recovery**:
- Atomic file operations use rename for atomicity
- Failed replacements attempt to restore original from `.orig` backup
- Job state is persisted before and after critical operations
- Temporary files are cleaned up on failure

### TUI Error Handling

**Startup Errors**:
- Job state directory not readable: Display error message and exit
- Terminal too small: Display warning and minimum size requirements

**Runtime Errors**:
- JSON parse failure: Skip invalid job file, log warning
- System metrics unavailable: Display "N/A" for affected metrics
- File I/O errors: Display error message, continue operation

## Testing Strategy

### Unit Testing

Unit tests will verify specific examples and edge cases:

**Configuration Module**:
- Valid TOML parsing with various field combinations
- Default value application when fields are missing
- Error handling for invalid TOML syntax

**Probe Module**:
- FFprobe JSON parsing with various stream configurations
- Main stream selection with and without default disposition
- Handling of missing or malformed metadata fields

**Classification Module**:
- Keyword detection in various path formats
- Score calculation with different metadata combinations
- Tie-breaking behavior when scores are equal

**Gates Module**:
- Size threshold boundary conditions
- Codec detection with various codec names
- Skip marker detection

**Encoder Modules**:
- CRF selection for boundary resolutions (1079, 1080, 1081, etc.)
- Preset selection with different quality tiers
- Command construction with various source classifications

**Size Gate Module**:
- Boundary conditions at exactly max_size_ratio
- Compression ratio calculation accuracy

**Sidecar Module**:
- File creation in various directory scenarios
- Content writing and reading

### Property-Based Testing

Property-based tests will verify universal properties across all inputs using the **proptest** crate for Rust. Each test will run a minimum of 100 iterations with randomly generated inputs.

**Testing Framework**: proptest 1.4+

**Property Test Configuration**:
```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    // test implementation
}
```

**Key Property Tests**:

1. **FFmpeg version parsing** (Property 1): Generate random version strings, verify parsing correctness
2. **Encoder selection** (Property 2): Generate random encoder availability sets, verify selection hierarchy
3. **Configuration round-trip** (Property 3): Generate random configs, serialize to TOML, deserialize, verify equality
4. **File discovery** (Property 4): Generate random directory trees, verify all video files are found
5. **Skip marker enforcement** (Property 5): Generate random file sets with/without markers, verify skip behavior
6. **Stable file detection** (Property 6): Generate random size change scenarios, verify stability detection
7. **FFprobe parsing** (Property 7): Generate random valid FFprobe JSON, verify all fields extracted correctly
8. **Stream selection** (Property 8): Generate random stream sets, verify correct stream selected
9. **Size threshold** (Property 9): Generate random file sizes and thresholds, verify skip behavior
10. **AV1 detection** (Property 10): Generate random codec names, verify AV1 detection
11. **Classification scoring** (Property 11): Generate random paths and metadata, verify classification logic
12. **CRF selection** (Property 12): Generate random resolutions and bitrates, verify CRF values
13. **Preset selection** (Property 13): Generate random resolutions and quality tiers, verify presets
14. **Stream mapping** (Property 14): Generate random jobs, verify all required flags present
15. **WebLike flags** (Property 15): Generate random classifications, verify flag inclusion/exclusion
16. **Pad filter** (Property 16): Generate random dimensions and classifications, verify filter logic
17. **SVT-AV1 commands** (Property 17): Generate random jobs, verify all SVT-AV1 parameters
18. **libaom-av1 commands** (Property 18): Generate random jobs, verify all libaom-av1 parameters
19. **Stream copying** (Property 19): Generate random jobs, verify copy parameters
20. **Concurrent limiting** (Property 20): Generate random job counts, verify concurrency limits
21. **Status transitions** (Property 21): Generate random job state changes, verify valid transitions
22. **Output validation** (Property 22): Generate random validation scenarios, verify correct decisions
23. **Size gate** (Property 23): Generate random file sizes and ratios, verify gate logic
24. **Atomic replacement** (Property 24): Generate random replacement scenarios, verify atomicity
25. **Job persistence** (Property 25): Generate random jobs, serialize to JSON, deserialize, verify equality
26. **TUI job loading** (Property 26): Generate random job directories, verify all jobs loaded
27. **Statistics calculation** (Property 27): Generate random job sets, verify statistics accuracy
28. **Job filtering** (Property 28): Generate random job sets and filters, verify filtering correctness
29. **Sort cycling** (Property 29): Generate random sort states, verify cycle progression
30. **Rate calculation** (Property 30): Generate random size measurements, verify rate accuracy
31. **ETA estimation** (Property 31): Generate random rates and sizes, verify ETA calculation
32. **Stage detection** (Property 32): Generate random job states, verify stage identification
33. **Responsive layout** (Property 33): Generate random terminal widths, verify column selection

**Generator Strategies**:
- Use proptest's built-in generators for primitives (integers, strings, booleans)
- Create custom generators for domain types (Job, ProbeResult, SourceClassification)
- Use `prop_oneof!` for enum generation
- Use `prop_compose!` for complex struct generation
- Constrain generators to valid input ranges (e.g., resolutions 480-4320, CRF 18-28)

### Integration Testing

Integration tests will verify end-to-end workflows:

**Daemon Integration Tests**:
- Full scan → probe → classify → encode → validate → replace workflow
- Error recovery scenarios (encoding failure, validation failure, size gate rejection)
- Concurrent job execution with various max_concurrent_jobs settings
- Sidecar file creation and skip marker enforcement across scan cycles

**TUI Integration Tests**:
- Job loading and display with various job states
- Filter and sort operations on real job data
- Statistics calculation with mixed job outcomes
- Responsive layout adaptation to terminal size changes

### Test Data

**Fixtures**:
- Sample FFprobe JSON outputs for various video formats
- Sample configuration files with different settings
- Mock directory structures for scanning tests

**Test Utilities**:
- FFmpeg/FFprobe mock implementations for unit tests
- Temporary directory creation and cleanup helpers
- Job factory functions for generating test jobs

## Deployment

### Debian Container Configuration

**Base Image**: `debian:bookworm-slim`

**System Dependencies**:
```dockerfile
RUN apt-get update && apt-get install -y \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*
```

**Binary Installation**:
```dockerfile
COPY target/release/av1d /usr/local/bin/
COPY target/release/av1top /usr/local/bin/
RUN chmod +x /usr/local/bin/av1d /usr/local/bin/av1top
```

**Configuration**:
```dockerfile
RUN mkdir -p /etc/av1d /var/lib/av1d/jobs /var/lib/av1d/temp
COPY config.toml /etc/av1d/config.toml
```

**Systemd Service** (`/etc/systemd/system/av1d.service`):
```ini
[Unit]
Description=AV1 Re-encoding Daemon
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/av1d --config /etc/av1d/config.toml
Restart=on-failure
RestartSec=10
User=av1d
Group=av1d

[Install]
WantedBy=multi-user.target
```

**User and Permissions**:
```bash
useradd -r -s /bin/false av1d
chown -R av1d:av1d /var/lib/av1d
```

### Build Configuration

**Cargo.toml** (workspace root):
```toml
[workspace]
members = ["crates/daemon", "crates/cli-daemon", "crates/cli-tui"]

[workspace.dependencies]
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.10", features = ["v4", "serde"] }
toml = "0.8"
proptest = "1.4"
```

**Release Build**:
```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

**Static Linking** (optional, for portability):
```bash
cargo build --release --target x86_64-unknown-linux-musl
```

### Runtime Configuration

**Default Configuration** (`/etc/av1d/config.toml`):
```toml
library_roots = ["/media"]
min_bytes = 2147483648  # 2 GiB
max_size_ratio = 0.90
scan_interval_secs = 60
job_state_dir = "/var/lib/av1d/jobs"
temp_output_dir = "/var/lib/av1d/temp"
max_concurrent_jobs = 1
prefer_encoder = "svt"
quality_tier = "high"
keep_original = false
write_why_sidecars = true
```

### Monitoring

**TUI Access**:
```bash
av1top --job-state-dir /var/lib/av1d/jobs --temp-output-dir /var/lib/av1d/temp
```

**Logs**:
- Daemon logs: `journalctl -u av1d -f`
- Job state: JSON files in `/var/lib/av1d/jobs/`
- Skip reasons: `.why.txt` files alongside video files

### Performance Tuning

**For 32-core EPYC**:
- Start with `max_concurrent_jobs = 1` for quality-first encoding
- Monitor CPU usage with TUI
- Increase to 2-3 if CPU utilization is low and quality is acceptable
- SVT-AV1 preset 3-4 should saturate cores well for 4K content

**Storage Considerations**:
- Place `temp_output_dir` on fast NVMe storage
- Ensure sufficient space (2x largest video file)
- Monitor disk I/O if encoding seems slow

**Memory Usage**:
- Expect 2-4 GB per concurrent job for 4K content
- Monitor with TUI system metrics
- Reduce `max_concurrent_jobs` if memory pressure occurs
