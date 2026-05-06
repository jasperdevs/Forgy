# Design notes

Forgy is intentionally a workflow layer over battle-tested media tools, not a codec project.

Source patterns reviewed:

- FFprobe exposes nested stream and format data through machine-readable writers, including JSON. Forgy uses `ffprobe -show_format -show_streams -print_format json` for `forge inspect` instead of parsing human text.
- yt-dlp treats progress and post-processing as hookable events, with status dictionaries for downloads and postprocessors. Forgy mirrors that idea at the process boundary by planning commands first, then reporting commands, outputs, warnings, and duration.
- HandBrake keeps presets as named JSON-like encoding bundles with codec, audio, picture, subtitle, and container choices. Forgy keeps built-in creator presets small and explicit so they can be shown, scripted, and eventually exported.
- whisper.cpp ships a CLI-centered transcription workflow. Forgy keeps transcription behind a provider trait with a mock provider, an external command provider, and a whisper.cpp placeholder so paid APIs are never required.

Links:

- https://ffmpeg.org/ffprobe.html
- https://github.com/yt-dlp/yt-dlp/blob/master/yt_dlp/YoutubeDL.py
- https://github.com/HandBrake/HandBrake/blob/master/preset/preset_cli_default.json
- https://github.com/ggml-org/whisper.cpp/blob/master/examples/cli/cli.cpp
