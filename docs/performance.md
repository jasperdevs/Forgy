# Performance

Forgy should feel instant for planning and help output. Media work is bounded by ffmpeg, ffprobe, yt-dlp, disk, and codecs.

## Local check

Machine: Windows, PowerShell, release binary built with thin LTO and strip.

| Command | Time |
| --- | ---: |
| `target/release/forge.exe --help` | 27.9 ms |
| `target/release/forge.exe --version` | 8.2 ms |
| `ffmpeg -version` | 27.4 ms |
| `yt-dlp --help` | 873.8 ms |
| `target/release/forge.exe --json doctor` | 862.0 ms |

Values are 5-run averages from `scripts/benchmark.ps1`. `doctor` is intentionally slower because it starts ffmpeg, ffprobe, yt-dlp, and hardware-acceleration checks. Normal help/version startup does not do that.

## Reproduce

```powershell
scripts/benchmark.ps1
```

The script builds release and measures `forge`, `ffmpeg`, and `yt-dlp` startup paths when those tools are available.

For real media operations, run:

```powershell
scripts/benchmark-real-ops.ps1 -Network
```

That benchmark generates fixture media and compares Forgy commands against direct `ffmpeg`, `ffprobe`, and `yt-dlp` backend commands. Latest local results are in [benchmark-results.md](benchmark-results.md).
