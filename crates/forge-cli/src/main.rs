use std::{
    fs,
    io::{self, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use anyhow::{Context, Result, bail};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use console::style;
use forge_core::{
    ForgeReport, JobPlan, RunOptions, dir_size, input_extension, input_stem,
    safe_output_path_for_source,
};
use forge_media::PlanRequest;
use forge_transcribe::{MockProvider, NotesProvider, NotesRequest, TranscriptionProvider};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "forge",
    version,
    about = "A beautiful, fast, terminal-native media toolkit."
)]
struct Cli {
    #[arg(long, global = true, help = "Print machine-readable JSON.")]
    json: bool,
    #[arg(
        long,
        global = true,
        help = "Plan work without running media commands."
    )]
    dry_run: bool,
    #[arg(long, global = true, help = "Skip overwrite prompts and assume yes.")]
    yes: bool,
    #[arg(
        long,
        global = true,
        default_value = "forge-output",
        help = "Output root folder."
    )]
    output: Utf8PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Show duration, codecs, tracks, metadata, size, and recommended actions.")]
    Inspect { file: Utf8PathBuf },
    #[command(about = "Convert media to another common format.")]
    Convert {
        input: Utf8PathBuf,
        #[arg(long)]
        to: String,
    },
    #[command(about = "Shrink videos, images, or folders with sensible ffmpeg defaults.")]
    Compress {
        input: Utf8PathBuf,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        quality: Option<u8>,
        #[arg(long)]
        recursive: bool,
    },
    #[command(about = "Cut a time range or remove silence.")]
    Clip {
        input: Utf8PathBuf,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        range: Option<String>,
        #[arg(long)]
        remove_silence: bool,
    },
    #[command(about = "Resize video or images by width, height, or creator preset.")]
    Resize {
        input: Utf8PathBuf,
        #[arg(long)]
        width: Option<u32>,
        #[arg(long)]
        height: Option<u32>,
        #[arg(long)]
        preset: Option<String>,
    },
    #[command(about = "Crop to common aspect ratios such as 9:16, 1:1, or 16:9.")]
    Crop {
        input: Utf8PathBuf,
        #[arg(long)]
        aspect: String,
    },
    #[command(about = "Attach or burn caption files.")]
    Captions(CaptionsArgs),
    #[command(about = "Create transcripts through the local provider abstraction.")]
    Transcribe {
        input: Utf8PathBuf,
        #[arg(long)]
        srt: bool,
        #[arg(long)]
        recursive: bool,
    },
    #[command(about = "Extract a still, grid, or best-frame thumbnail.")]
    Thumbnail {
        input: Utf8PathBuf,
        #[arg(long)]
        at: Option<String>,
        #[arg(long)]
        grid: bool,
        #[arg(long)]
        best_frame: bool,
    },
    #[command(about = "Normalize, clean, silence-trim, or extract audio.")]
    Audio {
        input: Utf8PathBuf,
        #[arg(long)]
        normalize: bool,
        #[arg(long)]
        remove_silence: bool,
        #[arg(long)]
        clean: bool,
        #[arg(long)]
        extract: bool,
    },
    #[command(about = "Wrap yt-dlp with Forgy output folders and reporting.")]
    Youtube(YoutubeArgs),
    #[command(about = "Run common operations across a folder.")]
    Batch(BatchArgs),
    #[command(about = "List, show, or scaffold creator presets.")]
    Preset {
        #[command(subcommand)]
        command: PresetCommand,
    },
    #[command(about = "Generate local-provider-ready notes from media or transcripts.")]
    Notes {
        input: Utf8PathBuf,
        #[arg(long)]
        chapters: bool,
        #[arg(long)]
        summary: bool,
    },
    #[command(
        about = "Check ffmpeg, ffprobe, yt-dlp, transcription, hardware, and output permissions."
    )]
    Doctor,
}

#[derive(Args)]
struct CaptionsArgs {
    input: Utf8PathBuf,
    #[arg(long = "from")]
    transcript: Option<Utf8PathBuf>,
    #[arg(long)]
    srt: Option<Utf8PathBuf>,
    #[arg(long)]
    burn: bool,
    #[arg(long)]
    style: Option<String>,
}

#[derive(Args)]
struct YoutubeArgs {
    url: String,
    #[arg(long)]
    audio: bool,
    #[arg(long)]
    transcript: bool,
    #[arg(long)]
    thumbnail: bool,
    #[arg(long)]
    metadata: bool,
    #[arg(long)]
    archive: bool,
    #[arg(
        long,
        help = "Ask yt-dlp to extract metadata without downloading media."
    )]
    simulate: bool,
}

#[derive(Args)]
struct BatchArgs {
    folder: Utf8PathBuf,
    #[arg(long)]
    compress: bool,
    #[arg(long)]
    target: Option<String>,
    #[arg(long)]
    to: Option<String>,
    #[arg(long)]
    resize: Option<u32>,
    #[arg(long)]
    transcribe: bool,
    #[arg(long)]
    notes: bool,
    #[arg(long)]
    recursive: bool,
}

#[derive(Subcommand)]
enum PresetCommand {
    List,
    Show { name: String },
    Create { name: String },
}

#[derive(Debug, Serialize)]
struct DoctorCheck {
    name: &'static str,
    ok: bool,
    note: &'static str,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    ctrlc::set_handler(move || flag.store(true, Ordering::SeqCst)).ok();

    let cli = Cli::parse();
    let run = RunOptions {
        json: cli.json,
        dry_run: cli.dry_run,
        yes: cli.yes,
        output_root: cli.output,
    };
    dispatch(cli.command, run, cancelled)
}

fn dispatch(command: Commands, run: RunOptions, cancelled: Arc<AtomicBool>) -> Result<()> {
    match command {
        Commands::Inspect { file } => inspect(&file, &run),
        Commands::Convert { input, to } => run_plan(
            forge_media::plan_convert(req(&input, &run), &to)?,
            vec![input],
            &run,
            cancelled,
        ),
        Commands::Compress {
            input,
            target,
            quality,
            recursive,
        } => {
            if input.is_dir() || recursive {
                batch_compress(&input, target.as_deref(), quality, &run, cancelled)
            } else {
                run_plan(
                    forge_media::plan_compress(req(&input, &run), target.as_deref(), quality)?,
                    vec![input],
                    &run,
                    cancelled,
                )
            }
        }
        Commands::Clip {
            input,
            from,
            to,
            range,
            remove_silence,
        } => run_plan(
            forge_media::plan_clip(
                req(&input, &run),
                from.as_deref(),
                to.as_deref(),
                range.as_deref(),
                remove_silence,
            )?,
            vec![input],
            &run,
            cancelled,
        ),
        Commands::Resize {
            input,
            width,
            height,
            preset,
        } => {
            if input.is_dir() {
                let files = forge_batch::collect_media(input.as_str(), true)?;
                for file in files {
                    run_plan(
                        forge_media::plan_resize(
                            req(&file, &run),
                            width,
                            height,
                            preset.as_deref(),
                        )?,
                        vec![file],
                        &run,
                        cancelled.clone(),
                    )?;
                }
                Ok(())
            } else {
                run_plan(
                    forge_media::plan_resize(req(&input, &run), width, height, preset.as_deref())?,
                    vec![input],
                    &run,
                    cancelled,
                )
            }
        }
        Commands::Crop { input, aspect } => run_plan(
            forge_media::plan_crop(req(&input, &run), &aspect)?,
            vec![input],
            &run,
            cancelled,
        ),
        Commands::Captions(args) => captions(args, &run, cancelled),
        Commands::Transcribe {
            input,
            srt,
            recursive,
        } => transcribe(input, srt, recursive, &run),
        Commands::Thumbnail {
            input,
            at,
            grid,
            best_frame,
        } => run_plan(
            forge_media::plan_thumbnail(req(&input, &run), at.as_deref(), grid, best_frame)?,
            vec![input],
            &run,
            cancelled,
        ),
        Commands::Audio {
            input,
            normalize,
            remove_silence,
            clean,
            extract,
        } => run_plan(
            forge_media::plan_audio(req(&input, &run), normalize, remove_silence, clean, extract)?,
            vec![input],
            &run,
            cancelled,
        ),
        Commands::Youtube(args) => youtube(args, &run, cancelled),
        Commands::Batch(args) => batch(args, &run, cancelled),
        Commands::Preset { command } => preset(command, &run),
        Commands::Notes {
            input,
            chapters,
            summary,
        } => notes(input, chapters, summary, &run),
        Commands::Doctor => doctor(&run),
    }
}

fn req<'a>(input: &'a Utf8Path, run: &'a RunOptions) -> PlanRequest<'a> {
    PlanRequest { input, run }
}

fn inspect(file: &Utf8Path, run: &RunOptions) -> Result<()> {
    let info = forge_media::inspect_path(file)?;
    if run.json {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("{}", style("Forge inspect").bold());
        println!("{}", forge_media::inspect_summary(&info));
    }
    Ok(())
}

fn run_plan(
    plan: JobPlan,
    inputs: Vec<Utf8PathBuf>,
    run: &RunOptions,
    cancelled: Arc<AtomicBool>,
) -> Result<()> {
    info!(
        job = plan.job_name.as_str(),
        commands = plan.commands.len(),
        outputs = plan.outputs.len(),
        dry_run = run.dry_run,
        json = run.json,
        "planned job"
    );
    if run.json || run.dry_run {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!("{} {}", style("Plan").bold(), plan.job_name);
        for op in &plan.operations {
            println!("  - {op}");
        }
    }
    if run.dry_run {
        return Ok(());
    }
    confirm_overwrites(&plan, run)?;

    let started = Utc::now();
    let timer = Instant::now();
    let size_before = dir_size(&inputs);
    let pb = ProgressBar::new(plan.commands.len() as u64);
    pb.set_style(ProgressStyle::with_template(
        "{spinner:.green} {wide_msg} [{pos}/{len}]",
    )?);
    let mut commands_run = Vec::new();
    let job_name = plan.job_name.clone();
    for command in &plan.commands {
        if cancelled.load(Ordering::SeqCst) {
            bail!("cancelled before running `{}`", command.display());
        }
        let command_line = command.display();
        info!(
            job = job_name.as_str(),
            command = %command_line,
            "running command"
        );
        pb.set_message(command_line.clone());
        command.run()?;
        info!(
            job = job_name.as_str(),
            command = %command_line,
            "finished command"
        );
        commands_run.push(command_line);
        pb.inc(1);
    }
    pb.finish_with_message("done");
    let size_after = dir_size(&plan.outputs);
    let job_dir = plan.job_dir.clone();
    let report = ForgeReport {
        job_name: plan.job_name,
        started_at: started,
        duration_ms: timer.elapsed().as_millis(),
        input_files: inputs,
        output_files: plan.outputs,
        operations_performed: plan.operations,
        commands_run,
        size_before_bytes: size_before,
        size_after_bytes: size_after,
        quality_settings: Vec::new(),
        warnings: plan.warnings,
    };
    forge_report::write_report_to(&job_dir, &report)?;
    if run.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "{} {}",
            style("Wrote").green().bold(),
            report
                .output_files
                .first()
                .map(|p| p.as_str())
                .unwrap_or("output")
        );
    }
    Ok(())
}

fn confirm_overwrites(plan: &JobPlan, run: &RunOptions) -> Result<()> {
    let existing: Vec<_> = plan.outputs.iter().filter(|path| path.exists()).collect();
    if existing.is_empty() || run.yes {
        return Ok(());
    }
    eprintln!("Forge will overwrite:");
    for path in &existing {
        eprintln!("  {}", path);
    }
    eprint!("Continue? [y/N] ");
    io::stderr().flush().ok();
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        bail!("aborted before overwriting existing output");
    }
    Ok(())
}

fn batch_compress(
    folder: &Utf8Path,
    target: Option<&str>,
    quality: Option<u8>,
    run: &RunOptions,
    cancelled: Arc<AtomicBool>,
) -> Result<()> {
    let files = forge_batch::collect_media(folder.as_str(), true)?;
    for file in files {
        run_plan(
            forge_media::plan_compress(req(&file, run), target, quality)?,
            vec![file],
            run,
            cancelled.clone(),
        )?;
    }
    Ok(())
}

fn captions(args: CaptionsArgs, run: &RunOptions, cancelled: Arc<AtomicBool>) -> Result<()> {
    let source = args
        .srt
        .or(args.transcript)
        .context("pass --from transcript.txt or --srt captions.srt")?;
    let mut plan = JobPlan::new(
        "captions",
        forge_core::prepare_job_dir(Path::new(run.output_root.as_str()), "captions")?,
    );
    let ext = input_extension(args.input.as_str(), "mp4");
    let output =
        safe_output_path_for_source(Path::new(plan.job_dir.as_str()), args.input.as_str(), &ext)?;
    let subtitle_filter = if args.burn {
        format!("subtitles='{}'", ffmpeg_filter_path(&source))
    } else {
        String::new()
    };
    let mut cmd = forge_core::CommandSpec::new("ffmpeg").args([
        "-hide_banner",
        "-y",
        "-i",
        args.input.as_str(),
    ]);
    if args.burn {
        cmd = cmd.args(["-vf", &subtitle_filter]);
    }
    cmd = cmd.args(["-c:a", "copy", output.as_str()]);
    plan.operations.push(format!("captions from {source}"));
    if let Some(style) = args.style {
        plan.operations.push(format!("caption style {style}"));
    }
    plan.outputs.push(output);
    plan.commands.push(cmd);
    run_plan(plan, vec![args.input], run, cancelled)
}

fn transcribe(input: Utf8PathBuf, srt: bool, recursive: bool, run: &RunOptions) -> Result<()> {
    let started = Utc::now();
    let timer = Instant::now();
    let files = if input.is_dir() || recursive {
        forge_batch::collect_media(input.as_str(), recursive)?
    } else {
        vec![input]
    };
    let size_before = dir_size(&files);
    let provider = MockProvider;
    let job_dir = forge_core::prepare_job_dir(Path::new(run.output_root.as_str()), "transcribe")?;
    let mut outputs = Vec::new();
    for file in &files {
        let transcript = provider.transcribe(file, srt)?;
        let stem = input_stem(file.as_str());
        let txt = job_dir.join(format!("{stem}.txt"));
        fs::write(&txt, transcript.text)?;
        outputs.push(txt);
        if let Some(srt_text) = transcript.srt {
            let srt_path = job_dir.join(format!("{stem}.srt"));
            fs::write(&srt_path, srt_text)?;
            outputs.push(srt_path);
        }
    }
    let report = ForgeReport {
        job_name: "transcribe".to_string(),
        started_at: started,
        duration_ms: timer.elapsed().as_millis(),
        input_files: files,
        output_files: outputs.clone(),
        operations_performed: vec![format!(
            "transcribe with {} provider",
            TranscriptionProvider::name(&provider)
        )],
        commands_run: Vec::new(),
        size_before_bytes: size_before,
        size_after_bytes: dir_size(&outputs),
        quality_settings: Vec::new(),
        warnings: vec![
            "mock provider output is for workflow testing; configure a real local provider for production transcripts"
                .to_string(),
        ],
    };
    forge_report::write_report_to(&job_dir, &report)?;
    if run.json {
        println!("{}", serde_json::to_string_pretty(&outputs)?);
    } else {
        println!("{} {}", style("Transcribed").green().bold(), outputs.len());
    }
    Ok(())
}

fn youtube(args: YoutubeArgs, run: &RunOptions, cancelled: Arc<AtomicBool>) -> Result<()> {
    if !forge_media::tool_available("yt-dlp") {
        bail!(
            "yt-dlp is not installed. Install it from https://github.com/yt-dlp/yt-dlp or with `pipx install yt-dlp`."
        )
    }
    let job_dir = forge_core::prepare_job_dir(Path::new(run.output_root.as_str()), "youtube")?;
    let template = job_dir.join("%(title).180B [%(id)s].%(ext)s");
    let mut cmd = forge_core::CommandSpec::new("yt-dlp").args([
        "--paths",
        job_dir.as_str(),
        "-o",
        template.as_str(),
    ]);
    if args.audio {
        cmd = cmd.args(["-x", "--audio-format", "mp3"]);
    }
    if args.transcript {
        cmd = cmd.args(["--write-subs", "--write-auto-subs", "--sub-format", "srt"]);
    }
    if args.thumbnail {
        cmd = cmd.arg("--write-thumbnail");
    }
    if args.metadata {
        cmd = cmd.arg("--write-info-json");
    }
    if args.archive {
        cmd = cmd.args(["--download-archive", job_dir.join("archive.txt").as_str()]);
    }
    if args.simulate {
        cmd = cmd.arg("--simulate");
    }
    cmd = cmd.arg(args.url);
    let mut plan = JobPlan::new("youtube", job_dir);
    plan.operations
        .push("youtube download workflow".to_string());
    plan.commands.push(cmd);
    run_plan(plan, Vec::new(), run, cancelled)
}

fn batch(args: BatchArgs, run: &RunOptions, cancelled: Arc<AtomicBool>) -> Result<()> {
    let files = forge_batch::collect_media(args.folder.as_str(), args.recursive)?;
    for file in files {
        if let Some(to) = &args.to {
            run_plan(
                forge_media::plan_convert(req(&file, run), to)?,
                vec![file.clone()],
                run,
                cancelled.clone(),
            )?;
        }
        if args.compress {
            run_plan(
                forge_media::plan_compress(req(&file, run), args.target.as_deref(), None)?,
                vec![file.clone()],
                run,
                cancelled.clone(),
            )?;
        }
        if let Some(width) = args.resize {
            run_plan(
                forge_media::plan_resize(req(&file, run), Some(width), None, None)?,
                vec![file.clone()],
                run,
                cancelled.clone(),
            )?;
        }
        if args.transcribe {
            transcribe(file.clone(), false, false, run)?;
        }
        if args.notes {
            notes(file, false, true, run)?;
        }
    }
    Ok(())
}

fn preset(command: PresetCommand, run: &RunOptions) -> Result<()> {
    match command {
        PresetCommand::List => {
            let presets = forge_presets::builtins();
            if run.json {
                println!("{}", serde_json::to_string_pretty(&presets)?);
            } else {
                for preset in presets {
                    println!("{:<14} {}", preset.name, preset.description);
                }
            }
        }
        PresetCommand::Show { name } => {
            let preset =
                forge_presets::find(&name).with_context(|| format!("unknown preset `{name}`"))?;
            println!("{}", serde_json::to_string_pretty(&preset)?);
        }
        PresetCommand::Create { name } => {
            println!(
                "Preset `{name}` template:\n{{\n  \"name\": \"{name}\",\n  \"container\": \"mp4\",\n  \"video_codec\": \"libx264\",\n  \"audio_codec\": \"aac\"\n}}"
            );
        }
    }
    Ok(())
}

fn notes(input: Utf8PathBuf, chapters: bool, summary: bool, run: &RunOptions) -> Result<()> {
    let started = Utc::now();
    let timer = Instant::now();
    let size_before = fs::metadata(&input).map(|meta| meta.len()).unwrap_or(0);
    let job_dir = forge_core::prepare_job_dir(Path::new(run.output_root.as_str()), "notes")?;
    let output = job_dir.join(format!("{}-notes.md", input_stem(input.as_str())));
    let provider = MockProvider;
    let document = provider.notes(NotesRequest {
        source: input.to_string(),
        chapters,
        summary,
        transcript: None,
    })?;
    fs::write(&output, document.markdown)?;
    let outputs = vec![output.clone()];
    let report = ForgeReport {
        job_name: "notes".to_string(),
        started_at: started,
        duration_ms: timer.elapsed().as_millis(),
        input_files: vec![input],
        output_files: outputs.clone(),
        operations_performed: vec![format!(
            "generate notes with {} provider",
            NotesProvider::name(&provider)
        )],
        commands_run: Vec::new(),
        size_before_bytes: size_before,
        size_after_bytes: dir_size(&outputs),
        quality_settings: Vec::new(),
        warnings: vec![
            "mock notes are placeholders until a local AI provider is configured".to_string(),
        ],
    };
    forge_report::write_report_to(&job_dir, &report)?;
    if run.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{} {output}", style("Wrote").green().bold());
    }
    Ok(())
}

fn doctor(run: &RunOptions) -> Result<()> {
    let hwaccels = detect_hwaccels();
    let checks = vec![
        DoctorCheck {
            name: "ffmpeg",
            ok: forge_media::tool_available("ffmpeg"),
            note: "required for media operations",
        },
        DoctorCheck {
            name: "ffprobe",
            ok: forge_media::tool_available("ffprobe"),
            note: "required for inspect",
        },
        DoctorCheck {
            name: "yt-dlp",
            ok: forge_media::tool_available("yt-dlp"),
            note: "optional for forge youtube",
        },
        DoctorCheck {
            name: "whisper provider",
            ok: forge_transcribe::WhisperCppProvider.available(),
            note: "optional local transcription",
        },
    ];
    let writable = fs::create_dir_all(&run.output_root).is_ok();
    if run.json {
        println!(
            "{}",
            serde_json::json!({ "checks": checks, "output_directory_writable": writable, "hardware_acceleration": hwaccels })
        );
    } else {
        println!("{}", style("Forge doctor").bold());
        for check in checks {
            let mark = if check.ok {
                style("ok").green()
            } else {
                style("missing").red()
            };
            println!("{:<18} {:<8} {}", check.name, mark, check.note);
        }
        println!(
            "{:<18} {:<8} {}",
            "output directory",
            if writable {
                style("ok").green()
            } else {
                style("blocked").red()
            },
            run.output_root
        );
        let hw = if hwaccels.is_empty() {
            "none reported by ffmpeg".to_string()
        } else {
            hwaccels.join(", ")
        };
        println!("{:<18} {}", "hardware accel", hw);
    }
    Ok(())
}

fn detect_hwaccels() -> Vec<String> {
    let Ok(output) = forge_core::CommandSpec::new("ffmpeg")
        .arg("-hwaccels")
        .run()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && *line != "Hardware acceleration methods:")
        .map(ToOwned::to_owned)
        .collect()
}

fn ffmpeg_filter_path(path: &Utf8Path) -> String {
    path.as_str()
        .replace('\\', "/")
        .replace(':', "\\:")
        .replace('\'', "\\'")
}
