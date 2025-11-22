# Performance Tuning Guide

This guide provides detailed recommendations for optimizing the AV1 Re-encoding Daemon for your hardware, particularly focusing on 32-core AMD EPYC processors.

## Table of Contents

- [Hardware Considerations](#hardware-considerations)
- [EPYC-Specific Recommendations](#epyc-specific-recommendations)
- [Quality vs Speed Tradeoffs](#quality-vs-speed-tradeoffs)
- [Storage Considerations](#storage-considerations)
- [Memory Optimization](#memory-optimization)
- [Concurrent Job Tuning](#concurrent-job-tuning)
- [Encoder Selection](#encoder-selection)
- [Monitoring and Profiling](#monitoring-and-profiling)
- [Optimization Strategies](#optimization-strategies)

## Hardware Considerations

### CPU Requirements

**Minimum:**
- 4 cores / 8 threads
- 2.5 GHz base clock
- AVX2 support

**Recommended:**
- 16+ cores / 32+ threads
- 3.0+ GHz base clock
- AVX2 or AVX-512 support

**Optimal (32-core EPYC):**
- 32 cores / 64 threads
- High core count for parallel processing
- Large L3 cache for encoding efficiency

### Memory Requirements

**Per Job Estimates:**
- 720p: 0.5-1 GB RAM
- 1080p: 1-2 GB RAM
- 1440p: 2-3 GB RAM
- 2160p (4K): 2-4 GB RAM
- 4320p (8K): 4-8 GB RAM

**System Overhead:**
- Daemon: ~50-100 MB
- TUI: ~20-50 MB
- OS and buffers: 2-4 GB

**Recommended Total RAM:**
- 1 concurrent job: 8 GB minimum, 16 GB recommended
- 2 concurrent jobs: 16 GB minimum, 32 GB recommended
- 3+ concurrent jobs: 32 GB minimum, 64 GB recommended

### Storage Requirements

**Disk Space:**
- Temporary files: 2x size of largest video file
- Job state: ~10-50 KB per job (negligible)
- Logs: 10-100 MB (depending on retention)

**I/O Performance:**
- HDD: 100-200 MB/s (acceptable for 1080p)
- SATA SSD: 500-550 MB/s (good for 4K)
- NVMe SSD: 2000-7000 MB/s (optimal for 4K and concurrent jobs)

## EPYC-Specific Recommendations

### Understanding EPYC Architecture

AMD EPYC processors use a chiplet design with multiple CCDs (Core Complex Dies):
- 32-core EPYC typically has 4 CCDs with 8 cores each
- Each CCD has its own L3 cache
- Inter-CCD communication has slight latency

### Optimal Configuration for 32-Core EPYC

**Starting Configuration (Maximum Quality):**
```toml
max_concurrent_jobs = 1
quality_tier = "very_high"
prefer_encoder = "svt"
```

**Why start with 1 job:**
- SVT-AV1 preset 3-4 will utilize 20-30 cores for 4K content
- Allows maximum quality with slower presets
- Prevents thread contention and cache thrashing
- Ensures consistent encoding quality

**Scaling Up (If CPU < 70% Utilized):**
```toml
max_concurrent_jobs = 2
quality_tier = "high"
prefer_encoder = "svt"
```

**Aggressive Throughput (Quality Compromise):**
```toml
max_concurrent_jobs = 3
quality_tier = "high"
prefer_encoder = "svt"
```

### NUMA Considerations

EPYC processors may have NUMA (Non-Uniform Memory Access) topology:

**Check NUMA configuration:**
```bash
numactl --hardware
lscpu | grep NUMA
```

**Optimize for NUMA:**
```bash
# Run daemon with NUMA awareness
numactl --interleave=all /usr/local/bin/av1d --config /etc/av1d/config.toml
```

**Update systemd service:**
```ini
[Service]
ExecStart=/usr/bin/numactl --interleave=all /usr/local/bin/av1d --config /etc/av1d/config.toml
```

### CPU Frequency Scaling

**Check current governor:**
```bash
cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
```

**Set performance governor for encoding:**
```bash
# Temporary (until reboot)
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Permanent (add to /etc/rc.local or systemd service)
```

**Why performance governor:**
- Prevents frequency throttling during encoding
- Maintains consistent encoding speed
- Reduces encoding time by 10-20%

### Thermal Management

**Monitor temperatures:**
```bash
# Install sensors
sudo apt install lm-sensors
sudo sensors-detect

# View temperatures
sensors

# Monitor in real-time
watch -n 1 sensors
```

**Thermal throttling prevention:**
- Ensure adequate cooling (especially for 32-core EPYC)
- Monitor CPU temperature during encoding
- Consider reducing `max_concurrent_jobs` if temps exceed 80°C
- Check data center cooling if in server environment

## Quality vs Speed Tradeoffs

### Quality Tiers

**Very High Quality (Slowest):**
```toml
quality_tier = "very_high"
max_concurrent_jobs = 1
```
- Preset reduced by 1 (e.g., preset 3 → preset 2 for 4K)
- 30-50% slower encoding
- 2-5% better compression efficiency
- Recommended for archival content

**High Quality (Recommended):**
```toml
quality_tier = "high"
max_concurrent_jobs = 1
```
- Standard presets (preset 3-5 based on resolution)
- Excellent quality-to-speed balance
- Recommended for most use cases

**Balanced (Faster):**
```toml
quality_tier = "high"
max_concurrent_jobs = 2
```
- Same quality per job, but parallel processing
- 2x throughput with 2 jobs
- Requires sufficient CPU and memory

### CRF Adjustments

The daemon automatically selects CRF based on resolution. To manually adjust:

**Lower CRF = Higher Quality, Larger Files:**
- Modify source code in `crates/daemon/src/encode/common.rs`
- Reduce CRF by 1-2 for archival quality
- Example: 2160p CRF 21 → CRF 19

**Higher CRF = Lower Quality, Smaller Files:**
- Increase CRF by 1-2 for faster encoding
- Example: 1080p CRF 23 → CRF 25
- Not recommended for quality-first encoding

### Preset Adjustments

**SVT-AV1 Presets (0-13):**
- Lower preset = slower, better quality
- Higher preset = faster, lower quality
- Default: 3-5 based on resolution

**Preset Performance Impact:**
- Preset 2 vs 3: 40-60% slower, 3-5% better compression
- Preset 3 vs 4: 30-40% slower, 2-3% better compression
- Preset 4 vs 5: 25-35% slower, 1-2% better compression

**When to adjust:**
- Archival content: Use preset 2-3 for all resolutions
- Fast turnaround: Use preset 5-6 for all resolutions
- Balanced: Use default presets (3-5)

### Encoder Comparison

**SVT-AV1 (Recommended):**
- Best quality-to-speed balance
- Excellent multi-threading (scales to 32+ cores)
- Fastest of the three encoders
- Preset 3-4 for 4K: ~0.5-1 fps on 32-core EPYC

**libaom-av1 (Fallback):**
- Reference encoder, highest quality potential
- Slower than SVT-AV1 (2-3x slower)
- Good multi-threading but less efficient
- cpu-used 3-4: ~0.2-0.5 fps on 32-core EPYC

**librav1e (Last Resort):**
- Rust-based encoder
- Slowest of the three
- Limited multi-threading
- Not recommended for production use

## Storage Considerations

### Temporary File Location

**Critical for Performance:**
```toml
temp_output_dir = "/path/to/fast/storage"
```

**Storage Tier Recommendations:**

1. **NVMe SSD (Optimal):**
   - 2000-7000 MB/s sequential write
   - Low latency for random I/O
   - Recommended for 4K encoding
   - Recommended for concurrent jobs

2. **SATA SSD (Good):**
   - 500-550 MB/s sequential write
   - Acceptable for 1080p and 1440p
   - May bottleneck 4K encoding

3. **HDD (Acceptable):**
   - 100-200 MB/s sequential write
   - Only for 720p and 1080p
   - Not recommended for 4K
   - Will bottleneck concurrent jobs

### Storage Layout

**Optimal Configuration:**
```
/fast/nvme/av1d/temp     → temp_output_dir (NVMe SSD)
/var/lib/av1d/jobs       → job_state_dir (any storage)
/media/library           → library_roots (any storage)
```

**Why separate temp storage:**
- Encoding writes continuously to temp file
- Fast storage reduces encoding bottleneck
- Prevents wearing out media library storage
- Allows different RAID/backup policies

### Disk Space Management

**Calculate required space:**
```bash
# Find largest video file
find /media -type f \( -name "*.mkv" -o -name "*.mp4" \) -exec du -h {} + | sort -rh | head -1

# Ensure temp_output_dir has 2x this space
df -h /var/lib/av1d/temp
```

**Automatic cleanup:**
- Daemon automatically deletes temp files on completion
- Failed encodes leave temp files for debugging
- Manually clean old temp files:
```bash
find /var/lib/av1d/temp -name "*.mkv.tmp" -mtime +7 -delete
```

### Network Storage Considerations

**NFS/SMB Mounted Libraries:**
- Acceptable for source files (read-only)
- NOT recommended for `temp_output_dir`
- Network latency will bottleneck encoding
- Use local storage for temp files

**Configuration for network storage:**
```toml
library_roots = ["/mnt/nfs/media"]  # Network storage OK
temp_output_dir = "/var/lib/av1d/temp"  # Local storage required
```

## Memory Optimization

### Memory Usage Patterns

**Per-Job Memory Breakdown:**
- FFmpeg process: 1-3 GB (depends on resolution)
- Input buffering: 100-500 MB
- Output buffering: 100-500 MB
- Encoder state: 500 MB - 2 GB

**System Memory Monitoring:**
```bash
# Watch memory usage
watch -n 1 free -h

# Per-process memory
ps aux --sort=-%mem | head -10

# Detailed memory breakdown
sudo smem -tk
```

### Preventing OOM (Out of Memory)

**Calculate safe concurrent jobs:**
```
Available RAM = Total RAM - OS overhead (4 GB)
Max concurrent jobs = Available RAM / (4 GB per 4K job)

Example: 64 GB RAM
Available = 64 - 4 = 60 GB
Max jobs = 60 / 4 = 15 jobs (theoretical)
Recommended = 3-4 jobs (practical with headroom)
```

**Set memory limits in systemd:**
```ini
[Service]
MemoryMax=32G
MemoryHigh=28G
```

**Monitor for memory pressure:**
```bash
# Check OOM killer logs
sudo journalctl -k | grep -i "out of memory"

# Monitor memory pressure
cat /proc/pressure/memory
```

### Swap Configuration

**Swap recommendations:**
- Encoding should NOT use swap (too slow)
- Configure swap as emergency fallback only
- Monitor swap usage: `free -h`

**If swap is being used:**
- Reduce `max_concurrent_jobs`
- Increase physical RAM
- Check for memory leaks

## Concurrent Job Tuning

### Finding Optimal Concurrency

**Step-by-step tuning process:**

1. **Start with 1 job:**
   ```toml
   max_concurrent_jobs = 1
   ```
   - Monitor CPU usage with TUI or `htop`
   - Note encoding speed (fps)
   - Baseline quality

2. **Increase to 2 jobs if CPU < 70%:**
   ```toml
   max_concurrent_jobs = 2
   ```
   - Monitor CPU usage (should be 80-95%)
   - Check encoding speed per job
   - Verify quality is acceptable

3. **Increase to 3 jobs if CPU < 80%:**
   ```toml
   max_concurrent_jobs = 3
   ```
   - Monitor CPU usage (should be 90-100%)
   - Check for thermal throttling
   - Verify memory usage < 80%

4. **Stop increasing if:**
   - CPU usage > 95% (diminishing returns)
   - Memory usage > 80% (risk of OOM)
   - CPU temperature > 80°C (thermal throttling)
   - Encoding speed per job drops > 20%

### Concurrency by Content Type

**4K Content (2160p):**
- Start: 1 job (preset 3 uses 20-30 cores)
- Max: 2 jobs on 32-core EPYC
- Memory: 4-8 GB per job

**1080p Content:**
- Start: 1 job (preset 4 uses 12-20 cores)
- Max: 2-3 jobs on 32-core EPYC
- Memory: 2-4 GB per job

**720p Content:**
- Start: 2 jobs (preset 5 uses 8-12 cores)
- Max: 3-4 jobs on 32-core EPYC
- Memory: 1-2 GB per job

**Mixed Content:**
- Use conservative setting (1-2 jobs)
- Daemon will process whatever is available
- Prevents resource exhaustion on 4K files

### Dynamic Concurrency (Future Enhancement)

Currently, `max_concurrent_jobs` is static. Future versions may support:
- Dynamic adjustment based on resolution
- CPU utilization feedback
- Memory pressure monitoring
- Thermal throttling detection

## Encoder Selection

### Encoder Performance Comparison

**SVT-AV1 (libsvtav1):**
- Speed: ★★★★★ (Fastest)
- Quality: ★★★★☆ (Excellent)
- Multi-threading: ★★★★★ (Excellent)
- Recommended for: All use cases

**libaom-av1:**
- Speed: ★★☆☆☆ (Slow)
- Quality: ★★★★★ (Best)
- Multi-threading: ★★★☆☆ (Good)
- Recommended for: Archival, when SVT unavailable

**librav1e:**
- Speed: ★☆☆☆☆ (Very slow)
- Quality: ★★★★☆ (Good)
- Multi-threading: ★★☆☆☆ (Limited)
- Recommended for: Last resort only

### When to Use Each Encoder

**Use SVT-AV1 when:**
- Available on your system (check with `ffmpeg -encoders | grep av1`)
- You want best quality-to-speed balance
- You have 16+ cores
- You're encoding 4K content

**Use libaom-av1 when:**
- SVT-AV1 is not available
- You need absolute maximum quality
- Encoding time is not a concern
- You're encoding archival content

**Use librav1e when:**
- Neither SVT-AV1 nor libaom-av1 is available
- You have no other choice
- Consider building FFmpeg with SVT-AV1 support instead

### Installing Encoders

**Check available encoders:**
```bash
ffmpeg -hide_banner -encoders | grep av1
```

**Install SVT-AV1 (Debian/Ubuntu):**
```bash
# From package manager (if available)
sudo apt install libsvtav1enc-dev

# Or build FFmpeg from source with SVT-AV1
# See: https://trac.ffmpeg.org/wiki/CompilationGuide
```

## Monitoring and Profiling

### Real-Time Monitoring

**Using the TUI:**
```bash
av1top
```
- CPU usage percentage
- Memory usage
- Active jobs and progress
- Encoding speed (fps)
- ETA for running jobs

**Using htop:**
```bash
htop -p $(pgrep -d',' av1d,ffmpeg)
```
- Per-core CPU usage
- Memory usage per process
- Thread count
- CPU affinity

**Using System Monitors:**
```bash
# CPU usage
mpstat 1

# Memory usage
vmstat 1

# Disk I/O
iostat -x 1

# Network (if using network storage)
iftop
```

### Performance Metrics

**Key Metrics to Track:**

1. **Encoding Speed (fps):**
   - 4K: 0.5-1.5 fps (SVT-AV1 preset 3-4)
   - 1080p: 2-5 fps (SVT-AV1 preset 4)
   - 720p: 5-10 fps (SVT-AV1 preset 5)

2. **CPU Utilization:**
   - Target: 80-95% for maximum efficiency
   - < 70%: Increase concurrent jobs
   - > 95%: May have diminishing returns

3. **Memory Usage:**
   - Target: < 80% of total RAM
   - > 80%: Reduce concurrent jobs
   - Swap usage: Should be 0

4. **Disk I/O:**
   - Write speed should match encoding bitrate
   - Monitor with `iostat -x 1`
   - High iowait: Storage bottleneck

5. **Throughput (GB/day):**
   - Calculate: (successful encodes * avg size) / time
   - Track over days/weeks
   - Optimize for your target throughput

### Profiling Encoding Performance

**Measure encoding time:**
```bash
# Time a single encode
time ffmpeg -i input.mkv -c:v libsvtav1 -crf 23 -preset 4 output.mkv

# Extract timing from logs
sudo journalctl -u av1d | grep "Job completed" | tail -20
```

**Analyze job statistics:**
```bash
# Average encoding time by resolution
jq -r 'select(.status == "Success") | "\(.video_height) \(.finished_at) \(.started_at)"' /var/lib/av1d/jobs/*.json

# Compression ratios
jq -r 'select(.status == "Success") | (.new_bytes / .original_bytes)' /var/lib/av1d/jobs/*.json | awk '{sum+=$1; count++} END {print sum/count}'

# Space saved
jq -r 'select(.status == "Success") | (.original_bytes - .new_bytes)' /var/lib/av1d/jobs/*.json | awk '{sum+=$1} END {print sum/1024/1024/1024 " GB"}'
```

## Optimization Strategies

### Progressive Optimization Approach

**Phase 1: Baseline (Week 1)**
```toml
max_concurrent_jobs = 1
quality_tier = "high"
prefer_encoder = "svt"
```
- Establish baseline performance
- Monitor CPU, memory, disk I/O
- Note encoding speeds and quality
- Identify bottlenecks

**Phase 2: Scale Concurrency (Week 2)**
```toml
max_concurrent_jobs = 2
quality_tier = "high"
prefer_encoder = "svt"
```
- Monitor resource utilization
- Compare encoding speed per job
- Check for thermal issues
- Verify quality is maintained

**Phase 3: Fine-Tune (Week 3+)**
- Adjust based on content mix
- Optimize storage layout
- Consider quality tier adjustments
- Monitor long-term stability

### Content-Specific Optimization

**Mostly 4K Content:**
```toml
max_concurrent_jobs = 1
quality_tier = "very_high"
min_bytes = 5368709120  # 5 GB
```

**Mostly 1080p Content:**
```toml
max_concurrent_jobs = 2
quality_tier = "high"
min_bytes = 2147483648  # 2 GB
```

**Mixed Content:**
```toml
max_concurrent_jobs = 1
quality_tier = "high"
min_bytes = 2147483648  # 2 GB
```

### Batch Processing Strategies

**Large Initial Backlog:**
1. Start with aggressive settings for throughput
2. Process bulk of library quickly
3. Re-encode priority content with higher quality later

**Ongoing Maintenance:**
1. Use quality-first settings
2. Process new content as it arrives
3. Monitor for consistent quality

### Quality Validation

**Periodic Quality Checks:**
```bash
# Sample random successful encodes
find /var/lib/av1d/jobs -name "*.json" -exec jq -r 'select(.status == "Success") | .source_path' {} \; | shuf | head -5

# Visually inspect samples
# Compare original vs encoded
# Verify quality is acceptable
```

**Automated Quality Metrics:**
```bash
# Calculate VMAF scores (requires ffmpeg with libvmaf)
ffmpeg -i original.mkv -i encoded.mkv -lavfi libvmaf -f null -

# Target VMAF scores:
# > 95: Excellent (visually lossless)
# 90-95: Very good (minor differences)
# 85-90: Good (acceptable for most content)
# < 85: Consider increasing quality
```

## Troubleshooting Performance Issues

### Slow Encoding Speed

**Symptoms:**
- Encoding speed < 0.5 fps for 4K
- Encoding speed < 2 fps for 1080p
- Jobs taking days to complete

**Possible Causes:**
1. **CPU bottleneck:**
   - Check CPU usage (should be 80-95%)
   - Verify CPU frequency (not throttled)
   - Check for thermal throttling

2. **Storage bottleneck:**
   - Check disk I/O with `iostat -x 1`
   - High iowait indicates storage issue
   - Move `temp_output_dir` to faster storage

3. **Memory bottleneck:**
   - Check for swap usage
   - Monitor memory pressure
   - Reduce concurrent jobs

4. **Encoder settings:**
   - Very low presets are very slow
   - Consider increasing preset by 1
   - Check if using libaom instead of SVT

**Solutions:**
- Increase preset (faster, slight quality loss)
- Move temp files to NVMe SSD
- Reduce concurrent jobs
- Enable CPU performance governor
- Check for background processes

### High Memory Usage

**Symptoms:**
- Memory usage > 80%
- Swap being used
- OOM killer terminating processes

**Solutions:**
- Reduce `max_concurrent_jobs`
- Increase physical RAM
- Check for memory leaks (restart daemon)
- Set systemd memory limits

### CPU Underutilization

**Symptoms:**
- CPU usage < 70%
- Cores sitting idle
- Slow overall throughput

**Solutions:**
- Increase `max_concurrent_jobs`
- Check if waiting for stable files
- Verify files meet `min_bytes` threshold
- Check for `.av1skip` markers

### Thermal Throttling

**Symptoms:**
- CPU temperature > 80°C
- Encoding speed decreases over time
- CPU frequency drops

**Solutions:**
- Improve cooling (fans, heatsink)
- Reduce `max_concurrent_jobs`
- Lower ambient temperature
- Check thermal paste application
- Consider undervolting CPU

## Conclusion

Performance tuning is an iterative process. Start conservative, monitor closely, and adjust based on your specific hardware and content mix. The goal is to maximize throughput while maintaining quality and system stability.

**Key Takeaways:**
- Start with 1 concurrent job for quality
- Monitor CPU, memory, and disk I/O
- Use NVMe SSD for temp files
- Scale concurrency based on utilization
- Validate quality periodically
- Adjust based on content mix

For questions or issues, refer to the main [README.md](README.md) and [DEPLOYMENT.md](DEPLOYMENT.md) documentation.
