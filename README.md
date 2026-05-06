# Forgy

[![CI](https://github.com/jasperdevs/Forgy/actions/workflows/ci.yml/badge.svg)](https://github.com/jasperdevs/Forgy/actions/workflows/ci.yml)

![Forgy imagegen logo](assets/logo-imagegen.png)

Forge is a beautiful, fast, terminal-native media toolkit.

It gives common media jobs one clean command: inspect, convert, compress, clip, resize, crop, captions, transcription, thumbnails, audio cleanup, YouTube downloads, batch jobs, presets, notes, and reports.

No desktop GUI. No web dashboard. No account. No cloud required.

```sh
cargo install --path crates/forge-cli
forge doctor
```

## Examples

```sh
forge inspect video.mp4
forge convert input.mov --to mp4
forge compress video.mp4 --target 50mb
forge clip video.mp4 --range 00:01:00..00:02:30
forge resize video.mp4 --width 1080
forge crop video.mp4 --aspect 9:16
forge thumbnail video.mp4 --at 00:00:15
forge audio podcast.mp3 --normalize --clean
forge youtube "https://www.youtube.com/watch?v=..." --audio --thumbnail
forge batch ./videos --compress --target 100mb --recursive
```

## Commands

| Command | Job |
| --- | --- |
| `inspect` | Read duration, format, streams, bitrate, metadata, size, and recommendations. |
| `convert` | Convert video, audio, or image formats through ffmpeg. |
| `compress` | Target file size or quality for files and folders. |
| `clip` | Cut ranges or remove silence. |
| `resize` / `crop` | Creator-safe dimensions and aspect ratios. |
| `captions` | Attach or burn subtitle files. |
| `transcribe` | Provider-backed transcript workflow with no paid API requirement. |
| `thumbnail` | Still, grid, or best-frame thumbnails. |
| `audio` | Normalize, clean, silence-trim, or extract audio. |
| `youtube` | yt-dlp downloads wrapped in Forgy output folders. |
| `batch` | Folder processing for common operations. |
| `preset` | Built-in creator export recipes. |
| `notes` | Local-provider-ready notes from media/transcripts. |
| `doctor` | Environment checks. |

## Output

Every job writes to:

```text
forge-output/<job-name>/
  output files
  forge-report.json
  forge-report.md
```

Reports include inputs, outputs, operations, commands run, duration, sizes, quality settings, and warnings.

## Built for scripting

```sh
forge inspect video.mp4 --json
forge compress video.mp4 --target 25mb --dry-run
forge convert input.wav --to mp3 --json
```

Global flags:

- `--json` for machine-readable output.
- `--dry-run` for output planning before execution.
- `--yes` for non-interactive automation.
- `--output <folder>` to change the output root.

## Presets

```sh
forge preset list
forge preset show tiktok
forge resize video.mp4 --preset youtube-short
```

Built-ins: `tiktok`, `reels`, `youtube`, `youtube-short`, `podcast`, `lecture`, `discord`, `web`, `archive`.

## Transcription and notes

Forgy has a provider abstraction for transcription and AI notes. The current implementation includes:

- mock provider for tests and local workflows
- external command provider
- whisper.cpp provider placeholder
- mock notes provider for local-provider-ready notes

Paid APIs are not required.

## Architecture

```text
crates/forge-cli         command parsing and terminal UX
crates/forge-core        shared planning, reports, output safety
crates/forge-media       ffmpeg and ffprobe workflows
crates/forge-presets     built-in creator presets
crates/forge-transcribe  provider abstraction
crates/forge-batch       folder discovery
crates/forge-report      JSON and Markdown reports
docs/                    design notes, presets, roadmap
examples/                copy-paste workflows
```

## Requirements

- Rust 1.95+
- ffmpeg and ffprobe for media work
- yt-dlp for `forge youtube`
- whisper.cpp or another local command for future real transcription

## Verification

```sh
cargo build
cargo test
forge doctor
```

See [docs/design-notes.md](docs/design-notes.md), [docs/presets.md](docs/presets.md), and [docs/roadmap.md](docs/roadmap.md).

Additional project notes:

- [Competitor comparison](docs/comparison.md)
- [Performance checks](docs/performance.md)
