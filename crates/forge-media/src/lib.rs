use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use camino::Utf8Path;
use forge_core::{
    CommandSpec, JobPlan, MediaInfo, RunOptions, format_duration, prepare_job_dir, safe_output_path,
};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct PlanRequest<'a> {
    pub input: &'a Utf8Path,
    pub run: &'a RunOptions,
}

pub fn tool_available(program: &str) -> bool {
    Command::new(program)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

pub fn inspect_path(path: &Utf8Path) -> Result<MediaInfo> {
    let output = CommandSpec::new("ffprobe")
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            path.as_str(),
        ])
        .run()
        .context("ffprobe inspect failed")?;
    let value: Value = serde_json::from_slice(&output.stdout)?;
    let streams = value
        .get("streams")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let format = value.get("format").cloned().unwrap_or(Value::Null);
    let file_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let duration = format
        .get("duration")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<f64>().ok());
    let bitrate = format
        .get("bit_rate")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<u64>().ok());
    let container = format
        .get("format_name")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let mut video_codecs = Vec::new();
    let mut audio_codecs = Vec::new();
    let mut audio_tracks = 0;
    let mut subtitle_tracks = 0;
    let mut resolution = None;
    for stream in streams {
        match stream.get("codec_type").and_then(Value::as_str) {
            Some("video") => {
                if let Some(codec) = stream.get("codec_name").and_then(Value::as_str) {
                    video_codecs.push(codec.to_string());
                }
                if resolution.is_none() {
                    if let (Some(w), Some(h)) = (
                        stream.get("width").and_then(Value::as_u64),
                        stream.get("height").and_then(Value::as_u64),
                    ) {
                        resolution = Some(format!("{w}x{h}"));
                    }
                }
            }
            Some("audio") => {
                audio_tracks += 1;
                if let Some(codec) = stream.get("codec_name").and_then(Value::as_str) {
                    audio_codecs.push(codec.to_string());
                }
            }
            Some("subtitle") => subtitle_tracks += 1,
            _ => {}
        }
    }
    let mut recommended_actions = Vec::new();
    if file_size > 100_000_000 {
        recommended_actions.push("try `forge compress` for a smaller shareable copy".to_string());
    }
    if !video_codecs.is_empty() && !video_codecs.iter().any(|c| c == "h264") {
        recommended_actions.push("convert to h264/mp4 for broad compatibility".to_string());
    }
    if audio_tracks > 0 {
        recommended_actions
            .push("use `forge audio --normalize` for spoken-word cleanup".to_string());
    }
    Ok(MediaInfo {
        path: path.to_owned(),
        duration_seconds: duration,
        format: container,
        resolution,
        video_codecs,
        audio_codecs,
        bitrate,
        audio_tracks,
        subtitle_tracks,
        metadata: value,
        file_size_bytes: file_size,
        recommended_actions,
    })
}

pub fn plan_convert(request: PlanRequest<'_>, to: &str) -> Result<JobPlan> {
    let job_dir = prepare_job_dir(Path::new(request.run.output_root.as_str()), "convert")?;
    let output = safe_output_path(Path::new(job_dir.as_str()), request.input.as_std_path(), to)?;
    let mut args = vec![
        "-hide_banner".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        request.input.to_string(),
    ];
    let ext = to.trim_start_matches('.').to_ascii_lowercase();
    match ext.as_str() {
        "mp4" => args.extend(
            ["-c:v", "libx264", "-c:a", "aac", "-movflags", "+faststart"]
                .into_iter()
                .map(String::from),
        ),
        "mp3" => args.extend(
            ["-vn", "-c:a", "libmp3lame", "-q:a", "2"]
                .into_iter()
                .map(String::from),
        ),
        "webp" => args.extend(
            ["-lossless", "0", "-quality", "82"]
                .into_iter()
                .map(String::from),
        ),
        "wav" => args.extend(["-c:a", "pcm_s16le"].into_iter().map(String::from)),
        _ => args.extend(["-c", "copy"].into_iter().map(String::from)),
    }
    args.push(output.to_string());
    let mut plan = JobPlan::new("convert", job_dir);
    plan.operations.push(format!("convert to {ext}"));
    plan.outputs.push(output);
    plan.commands.push(CommandSpec::new("ffmpeg").args(args));
    Ok(plan)
}

pub fn plan_compress(
    request: PlanRequest<'_>,
    target: Option<&str>,
    quality: Option<u8>,
) -> Result<JobPlan> {
    let job_dir = prepare_job_dir(Path::new(request.run.output_root.as_str()), "compress")?;
    let ext = request.input.extension().unwrap_or("mp4");
    let output = safe_output_path(
        Path::new(job_dir.as_str()),
        request.input.as_std_path(),
        ext,
    )?;
    let mut args = vec![
        "-hide_banner".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        request.input.to_string(),
    ];
    let info = inspect_path(request.input).ok();
    let is_image = matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "webp"
    );
    if is_image {
        args.extend([
            "-compression_level".to_string(),
            "6".to_string(),
            "-quality".to_string(),
            quality.unwrap_or(82).to_string(),
        ]);
    } else if let (Some(target), Some(_info), Some(duration)) = (
        target,
        info.as_ref(),
        info.as_ref().and_then(|i| i.duration_seconds),
    ) {
        let bytes = forge_core::parse_size_bytes(target)?;
        let video_kbps = (((bytes as f64 * 8.0) / duration) / 1000.0).max(256.0) as u64;
        args.extend([
            "-c:v".to_string(),
            "libx264".to_string(),
            "-b:v".to_string(),
            format!("{video_kbps}k"),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "128k".to_string(),
            "-movflags".to_string(),
            "+faststart".to_string(),
        ]);
    } else {
        args.extend([
            "-c:v".to_string(),
            "libx264".to_string(),
            "-crf".to_string(),
            quality.unwrap_or(24).to_string(),
            "-preset".to_string(),
            "medium".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "128k".to_string(),
        ]);
    }
    args.push(output.to_string());
    let mut plan = JobPlan::new("compress", job_dir);
    plan.operations.push(format!("compress {}", request.input));
    if target.is_some() {
        plan.operations
            .push(format!("target size {}", target.unwrap()));
    }
    plan.outputs.push(output);
    plan.commands.push(CommandSpec::new("ffmpeg").args(args));
    Ok(plan)
}

pub fn plan_clip(
    request: PlanRequest<'_>,
    from: Option<&str>,
    to: Option<&str>,
    range: Option<&str>,
    remove_silence: bool,
) -> Result<JobPlan> {
    let job_dir = prepare_job_dir(Path::new(request.run.output_root.as_str()), "clip")?;
    let ext = request.input.extension().unwrap_or("mp4");
    let output = safe_output_path(
        Path::new(job_dir.as_str()),
        request.input.as_std_path(),
        ext,
    )?;
    let (from, to) = parse_range(from, to, range);
    let mut args = vec!["-hide_banner".to_string(), "-y".to_string()];
    if let Some(from) = from.as_deref() {
        args.extend(["-ss".to_string(), from.to_string()]);
    }
    if let Some(to) = to.as_deref() {
        args.extend(["-to".to_string(), to.to_string()]);
    }
    args.extend(["-i".to_string(), request.input.to_string()]);
    if remove_silence {
        args.extend([
            "-af".to_string(),
            "silenceremove=start_periods=1:start_threshold=-45dB:detection=peak".to_string(),
        ]);
    }
    args.extend(["-c", "copy"].into_iter().map(String::from));
    args.push(output.to_string());
    let mut plan = JobPlan::new("clip", job_dir);
    plan.operations.push(format!("clip {}", request.input));
    if remove_silence {
        plan.operations.push("remove silence".to_string());
    }
    plan.outputs.push(output);
    plan.commands.push(CommandSpec::new("ffmpeg").args(args));
    Ok(plan)
}

pub fn plan_resize(
    request: PlanRequest<'_>,
    width: Option<u32>,
    height: Option<u32>,
    preset: Option<&str>,
) -> Result<JobPlan> {
    let mut width = width;
    let mut height = height;
    if let Some(name) = preset {
        if let Some(preset) = forge_presets::find(name) {
            width = width.or(preset.width);
            height = height.or(preset.height);
        } else {
            bail!("unknown preset `{name}`");
        }
    }
    let job_dir = prepare_job_dir(Path::new(request.run.output_root.as_str()), "resize")?;
    let ext = request.input.extension().unwrap_or("mp4");
    let output = safe_output_path(
        Path::new(job_dir.as_str()),
        request.input.as_std_path(),
        ext,
    )?;
    let scale = match (width, height) {
        (Some(w), Some(h)) => format!("scale={w}:{h}"),
        (Some(w), None) => format!("scale={w}:-2"),
        (None, Some(h)) => format!("scale=-2:{h}"),
        (None, None) => "scale=1080:-2".to_string(),
    };
    let mut plan = JobPlan::new("resize", job_dir);
    plan.operations.push(format!("resize with {scale}"));
    plan.outputs.push(output.clone());
    plan.commands.push(CommandSpec::new("ffmpeg").args([
        "-hide_banner",
        "-y",
        "-i",
        request.input.as_str(),
        "-vf",
        &scale,
        "-c:a",
        "copy",
        output.as_str(),
    ]));
    Ok(plan)
}

pub fn plan_crop(request: PlanRequest<'_>, aspect: &str) -> Result<JobPlan> {
    let job_dir = prepare_job_dir(Path::new(request.run.output_root.as_str()), "crop")?;
    let ext = request.input.extension().unwrap_or("mp4");
    let output = safe_output_path(
        Path::new(job_dir.as_str()),
        request.input.as_std_path(),
        ext,
    )?;
    let filter = crop_filter(aspect)?;
    let mut plan = JobPlan::new("crop", job_dir);
    plan.operations.push(format!("crop to {aspect}"));
    plan.outputs.push(output.clone());
    plan.commands.push(CommandSpec::new("ffmpeg").args([
        "-hide_banner",
        "-y",
        "-i",
        request.input.as_str(),
        "-vf",
        &filter,
        "-c:a",
        "copy",
        output.as_str(),
    ]));
    Ok(plan)
}

pub fn plan_thumbnail(
    request: PlanRequest<'_>,
    at: Option<&str>,
    grid: bool,
    best_frame: bool,
) -> Result<JobPlan> {
    let job_dir = prepare_job_dir(Path::new(request.run.output_root.as_str()), "thumbnail")?;
    let output = safe_output_path(
        Path::new(job_dir.as_str()),
        request.input.as_std_path(),
        "jpg",
    )?;
    let mut args = vec!["-hide_banner".to_string(), "-y".to_string()];
    if let Some(at) = at {
        args.extend(["-ss".to_string(), at.to_string()]);
    } else if best_frame {
        args.extend(["-ss".to_string(), "00:00:01".to_string()]);
    }
    args.extend(["-i".to_string(), request.input.to_string()]);
    if grid {
        args.extend(
            ["-vf", "fps=1/10,scale=320:-1,tile=3x3", "-frames:v", "1"]
                .into_iter()
                .map(String::from),
        );
    } else {
        args.extend(
            ["-frames:v", "1", "-q:v", "2"]
                .into_iter()
                .map(String::from),
        );
    }
    args.push(output.to_string());
    let mut plan = JobPlan::new("thumbnail", job_dir);
    plan.operations.push(
        if grid {
            "thumbnail grid"
        } else {
            "thumbnail still"
        }
        .to_string(),
    );
    plan.outputs.push(output);
    plan.commands.push(CommandSpec::new("ffmpeg").args(args));
    Ok(plan)
}

pub fn plan_audio(
    request: PlanRequest<'_>,
    normalize: bool,
    remove_silence: bool,
    clean: bool,
    extract: bool,
) -> Result<JobPlan> {
    let job_dir = prepare_job_dir(Path::new(request.run.output_root.as_str()), "audio")?;
    let output = safe_output_path(
        Path::new(job_dir.as_str()),
        request.input.as_std_path(),
        if extract {
            "mp3"
        } else {
            request.input.extension().unwrap_or("mp3")
        },
    )?;
    let mut filters = Vec::new();
    if normalize {
        filters.push("loudnorm");
    }
    if remove_silence {
        filters.push("silenceremove=start_periods=1:start_threshold=-45dB:detection=peak");
    }
    if clean {
        filters.push("highpass=f=80,lowpass=f=12000");
    }
    let mut args = vec![
        "-hide_banner".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        request.input.to_string(),
    ];
    if !filters.is_empty() {
        args.extend(["-af".to_string(), filters.join(",")]);
    }
    if extract {
        args.extend(
            ["-vn", "-c:a", "libmp3lame", "-q:a", "2"]
                .into_iter()
                .map(String::from),
        );
    }
    args.push(output.to_string());
    let mut plan = JobPlan::new("audio", job_dir);
    plan.operations.push("audio workflow".to_string());
    plan.outputs.push(output);
    plan.commands.push(CommandSpec::new("ffmpeg").args(args));
    Ok(plan)
}

pub fn inspect_summary(info: &MediaInfo) -> String {
    format!(
        "File: {}\nDuration: {}\nFormat: {}\nResolution: {}\nVideo: {}\nAudio: {} tracks ({})\nSubtitles: {} tracks\nBitrate: {}\nSize: {}\nRecommended:\n{}",
        info.path,
        format_duration(info.duration_seconds),
        info.format.as_deref().unwrap_or("unknown"),
        info.resolution.as_deref().unwrap_or("unknown"),
        if info.video_codecs.is_empty() {
            "none".to_string()
        } else {
            info.video_codecs.join(", ")
        },
        info.audio_tracks,
        if info.audio_codecs.is_empty() {
            "none".to_string()
        } else {
            info.audio_codecs.join(", ")
        },
        info.subtitle_tracks,
        info.bitrate
            .map(|b| b.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        forge_core::human_bytes(info.file_size_bytes),
        if info.recommended_actions.is_empty() {
            "- none".to_string()
        } else {
            info.recommended_actions
                .iter()
                .map(|a| format!("- {a}"))
                .collect::<Vec<_>>()
                .join("\n")
        },
    )
}

fn parse_range(
    from: Option<&str>,
    to: Option<&str>,
    range: Option<&str>,
) -> (Option<String>, Option<String>) {
    if let Some(range) = range {
        if let Some((a, b)) = range.split_once("..") {
            return (Some(a.to_string()), Some(b.to_string()));
        }
    }
    (from.map(ToOwned::to_owned), to.map(ToOwned::to_owned))
}

fn crop_filter(aspect: &str) -> Result<String> {
    match aspect {
        "9:16" => Ok("crop=ih*9/16:ih".to_string()),
        "1:1" => Ok("crop=min(iw\\,ih):min(iw\\,ih)".to_string()),
        "16:9" => Ok("crop=iw:iw*9/16".to_string()),
        other => bail!("unsupported crop aspect `{other}`. Try 9:16, 1:1, or 16:9"),
    }
}
