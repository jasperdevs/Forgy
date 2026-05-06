# Competitor comparison

Forgy is not trying to replace ffmpeg, HandBrake, yt-dlp, or whisper.cpp. It sits above them as a terminal-native workflow layer.

## What Forgy copies intentionally

### ffmpeg / ffprobe

Pattern used: keep codec work in ffmpeg and inspect through ffprobe JSON.

Why: ffprobe has a stable machine-readable JSON writer and exposes stream/format sections. Forgy uses this for `forge inspect` instead of scraping text output.

Source: https://ffmpeg.org/ffprobe.html

### yt-dlp

Pattern used: command planning, clean external-tool detection, archive support, metadata/subtitle/thumbnail flags, and progress/report hooks as a first-class concept.

Why: yt-dlp is strongest when it is scriptable and explicit about output templates and post-processing. Forgy wraps it rather than hiding that model.

Source: https://github.com/yt-dlp/yt-dlp/blob/master/yt_dlp/YoutubeDL.py

### HandBrake

Pattern used: named presets that encode container, codec, dimensions, and intent.

Why: creator exports are easier when people pick a named target like `youtube-short` or `discord` instead of building ffmpeg arguments from scratch.

Source: https://github.com/HandBrake/HandBrake/blob/master/preset/preset_cli_default.json

### whisper.cpp

Pattern used: local-first transcription provider, no paid API requirement, explicit future provider boundary.

Why: transcription should work as a local workflow. Forgy keeps the provider interface separate from media commands so agents and scripts can swap implementations later.

Source: https://github.com/ggml-org/whisper.cpp/blob/master/examples/cli/cli.cpp

## Current Forgy position

| Area | Competitor baseline | Forgy status |
| --- | --- | --- |
| Codecs | ffmpeg is the backend | Uses ffmpeg commands, does not fake codecs |
| Inspect | ffprobe JSON | Uses ffprobe JSON |
| Downloads | yt-dlp | Wraps yt-dlp when available |
| Presets | HandBrake preset model | Built-in creator presets |
| Transcription | whisper.cpp local CLI | Provider abstraction, mock and external command providers |
| Agent use | Scriptable commands and JSON | Global `--json`, `--dry-run`, reports |
| Reports | Usually tool-specific logs | Per-job JSON and Markdown reports |

## Gaps to close next

- Parse ffmpeg stderr progress into real percentage bars.
- Add user preset import/export.
- Add real whisper.cpp command configuration and model discovery.
- Add package installers and release artifacts.
- Add more cross-platform benchmarks.
