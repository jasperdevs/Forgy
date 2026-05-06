<p align="center">
  <img src="./assets/logo-imagegen.png" alt="Forgy logo" height="170">
</p>

<h1 align="center">Forgy</h1>

<p align="center">
  A fast, terminal-native media toolkit that ships as one executable: <code>forge</code>.
</p>

<p align="center">
  <a href="https://github.com/jasperdevs/Forgy/actions/workflows/ci.yml"><img src="https://github.com/jasperdevs/Forgy/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT license">
  <img src="https://img.shields.io/badge/rust-2024-orange" alt="Rust 2024">
  <img src="https://img.shields.io/badge/media-ffmpeg%20%2B%20yt--dlp-black" alt="ffmpeg and yt-dlp">
  <img src="https://img.shields.io/github/stars/jasperdevs/Forgy" alt="GitHub stars">
</p>

<div align="center">
  <a href="./docs/design-notes.md">Design</a>
  <span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
  <a href="./docs/presets.md">Presets</a>
  <span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
  <a href="./examples">Examples</a>
  <span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
  <a href="./docs/benchmark-results.md">Benchmarks</a>
  <span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
  <a href="./docs/roadmap.md">Roadmap</a>
  <span>&nbsp;&nbsp;•&nbsp;&nbsp;</span>
  <a href="https://github.com/jasperdevs/Forgy/issues">Issues</a>
</div>

## What is Forgy?

Forgy is a CLI-only workflow layer for everyday media work: inspect, convert, compress, clip, resize, crop, captions, transcription, thumbnails, audio cleanup, YouTube metadata/download flows, batch jobs, presets, notes, and reports.

It does not pretend to be a codec engine. Forgy plans the job, picks good defaults, runs proven tools like `ffmpeg`, `ffprobe`, and `yt-dlp`, then writes clean outputs and reports into `forge-output/`.

```sh
forge inspect video.mp4
forge convert input.mov --to mp4
forge compress video.mp4 --target 50mb
forge clip video.mp4 --range 00:01:00..00:02:30
```

## Install

Install the Rust binary through npm:

```sh
npm install -g forgy
forge doctor
```

Or build from source:

```sh
git clone https://github.com/jasperdevs/Forgy
cd Forgy
cargo install --path crates/forge-cli --locked
forge doctor
```

Requirements:

- Rust 1.95+
- `ffmpeg` and `ffprobe` for media commands
- `yt-dlp` for `forge youtube`
- A local transcription command only if you wire one in; paid APIs are not required

## Use it

Run `forge doctor` first. It checks `ffmpeg`, `ffprobe`, `yt-dlp`, transcription provider availability, hardware acceleration hints, and output permissions.

```sh
forge doctor
```

Then run the media job directly:

```sh
forge inspect video.mp4
forge thumbnail "https://example.com/video.mp4" --at 00:00:15
forge resize video.mp4 --preset youtube-short
forge captions video.mp4 --srt captions.srt --burn --style shorts
forge audio podcast.wav --normalize --clean
forge batch ./videos --compress --target 100mb --recursive
```

Use JSON and dry-run mode when an agent or script needs predictable output:

```sh
forge inspect video.mp4 --json
forge compress video.mp4 --target 25mb --dry-run
forge convert input.wav --to mp3 --json --yes
```

Local paths and direct media URLs are interchangeable for `ffmpeg`/`ffprobe` backed commands when the backend can read the URL. Use `forge youtube <url>` for YouTube-style pages that need `yt-dlp`.

## Commands

| Command | What it does |
| --- | --- |
| `forge inspect <file>` | Duration, format, resolution, codecs, bitrate, streams, metadata, size, and recommendations. |
| `forge convert <input> --to <format>` | Convert video, audio, or image formats through `ffmpeg`. |
| `forge compress <input>` | Compress files or folders by target size or quality. |
| `forge clip <input>` | Cut ranges or remove silence. |
| `forge resize <input>` | Resize files or folders by width, height, or preset. |
| `forge crop <input>` | Crop to creator-friendly aspect ratios. |
| `forge captions <input>` | Attach, create, or burn subtitle files. |
| `forge transcribe <input>` | Provider-backed transcription workflow with mock and external-command providers. |
| `forge thumbnail <input>` | Still frame, grid, or best-frame thumbnail generation. |
| `forge audio <input>` | Normalize, clean, silence-trim, or extract audio. |
| `forge youtube <url>` | `yt-dlp` workflow wrapper for audio, transcript, thumbnail, metadata, and archive flows. |
| `forge batch <folder>` | Folder processing for common operations. |
| `forge preset` | Show built-in creator export recipes. |
| `forge notes <input>` | Local-provider-ready notes from media or transcripts. |
| `forge doctor` | Environment checks. |

## Presets

Presets are readable export recipes, not hidden magic.

```sh
forge preset list
forge preset show tiktok
forge resize raw.mov --preset youtube-short
```

Built-ins: `tiktok`, `reels`, `social`, `youtube`, `youtube-short`, `podcast`, `lecture`, `discord`, `web`, `archive`.

## Output

Every run writes to a job folder by default:

```text
forge-output/<job-name>/
  output files
  forge-report.json
  forge-report.md
```

Reports include inputs, outputs, operations, commands run, duration, size before and after, quality settings, and warnings.

## Tested on real media

The repo includes a real smoke suite that generates video, audio, image, subtitle, transcript, and folder fixtures, then runs the CLI against them.

```powershell
scripts/verify-e2e.ps1 -Network
```

The current suite covers `doctor`, `inspect`, `convert`, `compress`, `clip`, `resize`, `crop`, `captions`, `transcribe`, `thumbnail`, `audio`, `batch`, `preset`, `notes`, and `youtube --simulate`.

There is also a real operation benchmark that compares Forgy commands against direct backend commands:

```powershell
scripts/benchmark-real-ops.ps1 -Network
```

Latest measured results are in [docs/benchmark-results.md](docs/benchmark-results.md).

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
```

CI runs format, build, clippy, tests, a release build, and the local real-media E2E suite on `main`.

## License

MIT. See [LICENSE](LICENSE).
