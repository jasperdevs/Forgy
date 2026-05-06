# Presets

Presets are readable export recipes. They are not magic; each one maps to ordinary ffmpeg settings.

Built-ins:

- `tiktok` - 1080x1920 vertical MP4, h264/aac, creator captions in mind.
- `reels` - 1080x1920 vertical MP4 for Instagram Reels.
- `youtube` - 1920x1080 h264/aac MP4 with quality-biased defaults.
- `youtube-short` - vertical YouTube Shorts export.
- `podcast` - spoken-word MP3 defaults.
- `lecture` - smaller readable screen or lecture capture.
- `discord` - aggressive compression for chat sharing.
- `web` - compact web-friendly media.
- `archive` - conservative copy/remux preference.

Commands:

```sh
forge preset list
forge preset show youtube
forge resize video.mp4 --preset tiktok
```
