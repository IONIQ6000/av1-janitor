# Rust AV1 Re-Encoder + ratatui TUI (Software-Only, **Quality‑First**, ffmpeg ≥ 8.0)

This is a fresh Rust spec, starting from scratch, **software AV1 only** (no GPU, no Docker, no static ffmpeg).  
It preserves the Tdarr workflow you built, but now defaults to **quality‑first settings across the board** while still targeting real size savings.

---

## 0. Goals (same flow, quality‑first defaults)

### Workflow invariants (must keep)
- **Skip small files** (default ≤ 2 GiB) to avoid AV1 making already‑low‑bitrate sources look worse.
- **Skip if already AV1.**
- **Stable‑file gate** so we don’t touch files mid‑transfer.
- **WebRip safety**: detect web‑style sources and apply timestamp / padding safeguards.
- **Drop Russian tracks**:
  - remove `ru` / `rus` **audio**
  - remove `ru` / `rus` **subtitles**
- **Convert video to AV1** at high visual quality.
- **Copy audio/subs otherwise** (no re‑encode).
- **Post‑encode size gate**:
  - If new ≥ `max_size_ratio` * original (default 0.90), **delete new**, keep original, and create `.av1skip`.
- **On success**: validate output then atomically replace original.
- **Sidecars**:
  - `.av1skip` permanent skip marker
  - `.why.txt` human reasons for skip/fail

### Quality‑first meaning
- Prefer **slower presets** / **higher efficiency modes** by default.
- Use **lower CRF targets** than the previous “balanced” spec.
- Keep multithreading (your EPYC) but avoid quality‑hurting parallel tricks unless required.

---

## 1. Runtime prerequisites

### 1.1 System ffmpeg ≥ 8.0
At daemon startup:

1. Run `ffmpeg -version`.
2. Parse major version from first line.
3. **Abort** if `major < 8`.

### 1.2 AV1 software encoders available
We require at least one:

1. **Preferred: `libsvtav1`**  
   Best speed/quality scaling for many‑core CPUs like EPYC. Slower presets improve quality and compression.  
2. **Fallback: `libaom-av1`**  
   Very high quality, slower; threading needs row‑mt + tiles. Tiles cause small efficiency losses, so keep tile counts modest.  
3. **Last resort: `librav1e`**  
   Acceptable but slow and limited scaling.

Startup check:
```bash
ffmpeg -hide_banner -encoders | grep -E "libsvtav1|libaom-av1|librav1e"
```

Fail fast if none are present.

---

## 2. Encoder selection strategy (quality‑first)

1. Use **SVT‑AV1** whenever available.
2. Else use **libaom‑av1**.
3. Else use **rav1e**.
4. Else fail.

Rationale: SVT‑AV1’s slower presets deliver better compression at the same CRF and scale well on many cores, making it the best “quality‑first but still practical” default.  

---

## 3. Threading on 32‑core EPYC (quality‑safe)

### 3.1 ffmpeg global
Always include:
```bash
-threads 0
```
Let ffmpeg/encoder auto‑use logical cores.

### 3.2 Cross‑title parallelism
Expose:
```toml
max_concurrent_jobs = 1  # default quality‑safe
```
You can raise this later to 2–3 to saturate EPYC, but quality settings stay the same (just more jobs at once).

### 3.3 Encoder‑specific

#### SVT‑AV1
SVT handles internal threading well. Optionally pass:
```bash
-svtav1-params "lp=0"
```
(auto logical processors).

#### libaom‑av1
`row-mt` + tiles are needed to scale threads, but tiles slightly reduce efficiency.  
So: **use the minimum tiles that still give good throughput**.

Defaults:
```bash
-row-mt 1 -tiles {TILES} -threads 32
```

Tile guidance (quality‑first):
- ≤1080p: `tiles 2x1`
- 1440p–2160p: `tiles 2x2`
- 8K: `tiles 3x2`

Avoid “4x4 tiles everywhere” because more tiles = more efficiency loss.

#### rav1e
Tile‑threading only; don’t expect EPYC saturation.

---

## 4. Quality policy (quality‑first ladder)

### 4.1 CRF ladder (lower = higher quality)
This ladder is **more conservative** than the balanced spec:

| Height | CRF (SVT/AOM) | Notes |
|---|---:|---|
| ≥2160p (4K) | 21–22 | Disc‑like sources prefer 21 |
| 1440p | 22–23 | |
| 1080p | 23–24 | |
| <1080p | 24–25 | don’t over‑compress low‑res |

Implementation rule:
- Start at the **higher‑quality end** of each range by default.
- If the original bitrate is already very low *for that resolution*, shift to the higher CRF within the range.

### 4.2 SVT‑AV1 presets (quality‑first)
Slower preset numbers produce better quality/compression at a fixed CRF.  
Default presets:

| Height | Preset |
|---|---:|
| ≥2160p | 3 |
| 1440p | 4 |
| 1080p | 4 |
| <1080p | 5 |

If you ever want “extreme quality”, allow:
```toml
quality_tier = "very_high"
```
which shifts presets down by 1 (e.g., 2/3/3/4).

### 4.3 libaom‑av1 speed knobs (fallback)
Quality‑first defaults:

- `-cpu-used 3` for ≥1440p
- `-cpu-used 4` for 1080p
- `-cpu-used 5` for <1080p

Always use:
```bash
-b:v 0 -crf {CRF}
```
for CRF‑based constant quality.

### 4.4 rav1e fallback (last resort)
Keep moderate‑quality settings; only used if SVT/AOM unavailable:
```bash
-qp 75 -speed 3 -threads 0
```

---

## 5. Software ffmpeg command templates

### 5.1 Shared mapping / filters

**Stream mapping**
Keep everything except:
- attached pictures
- Russian audio/subtitles

```bash
-map 0 \
-map -0:v:m:attached_pic \
-map 0:v:{VORD} \
-map 0:a? -map -0:a:m:language:ru -map -0:a:m:language:rus \
-map 0:s? -map -0:s:m:language:ru -map -0:s:m:language:rus \
-map_chapters 0 -map_metadata 0
```

**WebRip‑safe flags**
If classifier says WebLike:
```bash
-fflags +genpts -copyts -start_at_zero \
-vsync 0 -avoid_negative_ts make_zero
```

**Pad filter**
If WebLike **or** odd dimensions:
```bash
-vf "pad=ceil(iw/2)*2:ceil(ih/2)*2,setsar=1"
```

---

### 5.2 SVT‑AV1 (default path)

```bash
ffmpeg -hide_banner -y \
  {WEBSAFE_INPUT} \
  -i "{INPUT}" \
  -map 0 -map -0:v:m:attached_pic -map 0:v:{VORD} \
  -map 0:a? -map -0:a:m:language:ru -map -0:a:m:language:rus \
  -map 0:s? -map -0:s:m:language:ru -map -0:s:m:language:rus \
  -map_chapters 0 -map_metadata 0 \
  {PAD_FILTER} \
  -c:v libsvtav1 -crf {CRF} -preset {SVT_PRESET} -threads 0 \
  -svtav1-params "lp=0" \
  -c:a copy -c:s copy \
  -max_muxing_queue_size 2048 \
  {WEBSAFE_OUTPUT} \
  "{TMP_OUTPUT}"
```

---

### 5.3 libaom‑av1 (fallback, quality‑first)

```bash
ffmpeg -hide_banner -y \
  {WEBSAFE_INPUT} \
  -i "{INPUT}" \
  ...same map/filter as above... \
  -c:v libaom-av1 -b:v 0 -crf {CRF} -cpu-used {CPU_USED} \
  -row-mt 1 -tiles {TILES} -threads 32 \
  -c:a copy -c:s copy \
  -max_muxing_queue_size 2048 \
  {WEBSAFE_OUTPUT} \
  "{TMP_OUTPUT}"
```

---

### 5.4 rav1e (last resort)

```bash
ffmpeg -hide_banner -y \
  {WEBSAFE_INPUT} \
  -i "{INPUT}" \
  ...same map/filter as above... \
  -c:v librav1e -qp 75 -speed 3 -threads 0 \
  -c:a copy -c:s copy \
  -max_muxing_queue_size 2048 \
  {WEBSAFE_OUTPUT} \
  "{TMP_OUTPUT}"
```

---

## 6. Workflow (daemon rules)

1. **Recursive scan** of `library_roots`.
2. Skip if `.av1skip` exists or extension not allowed.
3. **Stable‑file gate** (size unchanged over 10s).
4. **Probe metadata** via local `ffprobe`.
5. **Gates**:
   - No video → skip.
   - Size ≤ min_bytes → skip.
   - Already AV1 → skip.
6. Pick **main video stream** (default disposition else first).
7. **Classify source** (scored WebLike/DiscLike/Unknown + reasons).
8. **Build encoder command** with quality‑first ladder.
9. **Run encode** to temp output in same directory.
10. **Validate output**:
    - ffprobe reads OK
    - EXACTLY one AV1 video stream
    - duration within epsilon of original
11. **Size gate**:
    - If new ≥ original * max_size_ratio:
      - delete new
      - write `.av1skip`
      - `.why.txt`
12. **Atomic replace**:
    - rename original → `.orig` (optional)
    - rename temp → original name
    - delete `.orig` if configured
13. Persist job state as JSON.

---

## 7. Rust workspace layout

```
av1d-rs/
├── Cargo.toml
├── crates/
│   ├── daemon/
│   ├── cli-daemon/
│   └── cli-tui/
└── README.md
```

Core modules in `daemon`:
- `config.rs`
- `scan.rs`
- `stable.rs`
- `probe.rs`
- `classify.rs`
- `encode/` (svt, aom, rav1e)
- `gates.rs`
- `size_gate.rs`
- `sidecars.rs`
- `jobs.rs`

All external processes via `tokio::process::Command`.

---

## 8. ratatui TUI (same as before)

`av1top` shows:
- CPU / memory bars
- Job table
- Status counts
- Keybinds: `q` quit, `r` refresh

---

## 9. Config knobs (quality‑first defaults)

```toml
library_roots = ["/media"]
min_bytes = 2147483648
max_size_ratio = 0.90
scan_interval_secs = 60
job_state_dir = "/var/lib/av1d/jobs"

max_concurrent_jobs = 1
prefer_encoder = "svt"         # svt|aom|rav1e
quality_tier = "high"          # high|very_high

keep_original = false
write_why_sidecars = true
```

---

## 10. Milestones

1. Workspace skeleton + config + jobs.
2. Scanner + stable‑file gate + `.av1skip`.
3. Local ffprobe parsing.
4. Gates + main‑video selection.
5. Web source classifier.
6. SVT encode path; fallback aom/rav1e.
7. Validate + size gate + atomic replace.
8. ratatui dashboard.
9. Concurrency tuning for EPYC.

---

This spec keeps your Tdarr logic but upgrades it to a **quality‑first AV1 software pipeline** tailored for a many‑core EPYC box.
