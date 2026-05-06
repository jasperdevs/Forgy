# Podcast cleanup

```sh
forge inspect episode.wav
forge audio episode.wav --normalize --clean
forge transcribe episode.wav --srt
forge notes episode.wav --chapters --summary
```
