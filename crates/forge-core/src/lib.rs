use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const DEFAULT_OUTPUT_DIR: &str = "forge-output";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOptions {
    pub json: bool,
    pub dry_run: bool,
    pub yes: bool,
    pub output_root: Utf8PathBuf,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            json: false,
            dry_run: false,
            yes: false,
            output_root: Utf8PathBuf::from(DEFAULT_OUTPUT_DIR),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn display(&self) -> String {
        let mut out = self.program.clone();
        for arg in &self.args {
            out.push(' ');
            out.push_str(&shell_quote(arg));
        }
        out
    }

    pub fn run(&self) -> Result<Output> {
        let mut cmd = Command::new(&self.program);
        cmd.args(self.args.iter().map(OsString::from));
        let output = cmd.output().with_context(|| {
            format!(
                "failed to start `{}`. Is it installed and available on PATH?",
                self.program
            )
        })?;
        if !output.status.success() {
            bail!(
                "`{}` failed with status {}\n{}",
                self.display(),
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(output)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobPlan {
    pub job_name: String,
    pub job_dir: Utf8PathBuf,
    pub operations: Vec<String>,
    pub commands: Vec<CommandSpec>,
    pub outputs: Vec<Utf8PathBuf>,
    pub warnings: Vec<String>,
}

impl JobPlan {
    pub fn new(job_name: impl Into<String>, job_dir: Utf8PathBuf) -> Self {
        Self {
            job_name: job_name.into(),
            job_dir,
            operations: Vec::new(),
            commands: Vec::new(),
            outputs: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeReport {
    pub job_name: String,
    pub started_at: DateTime<Utc>,
    pub duration_ms: u128,
    pub input_files: Vec<Utf8PathBuf>,
    pub output_files: Vec<Utf8PathBuf>,
    pub operations_performed: Vec<String>,
    pub commands_run: Vec<String>,
    pub size_before_bytes: u64,
    pub size_after_bytes: u64,
    pub quality_settings: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    pub path: Utf8PathBuf,
    pub duration_seconds: Option<f64>,
    pub format: Option<String>,
    pub resolution: Option<String>,
    pub video_codecs: Vec<String>,
    pub audio_codecs: Vec<String>,
    pub bitrate: Option<u64>,
    pub audio_tracks: usize,
    pub subtitle_tracks: usize,
    pub metadata: serde_json::Value,
    pub file_size_bytes: u64,
    pub recommended_actions: Vec<String>,
}

pub fn prepare_job_dir(root: &Path, job_name: &str) -> Result<Utf8PathBuf> {
    fs::create_dir_all(root).with_context(|| format!("failed to create {}", root.display()))?;
    let candidate = root.join(slug(job_name));
    let path = next_available_path(&candidate);
    fs::create_dir_all(&path).with_context(|| format!("failed to create {}", path.display()))?;
    Utf8PathBuf::from_path_buf(path).map_err(|p| anyhow::anyhow!("non-utf8 path {}", p.display()))
}

pub fn safe_output_path(job_dir: &Path, input: &Path, extension: &str) -> Result<Utf8PathBuf> {
    let stem = input_stem(input.to_string_lossy().as_ref());
    let filename = format!("{}.{}", stem, extension.trim_start_matches('.'));
    let path = next_available_path(&job_dir.join(filename));
    Utf8PathBuf::from_path_buf(path).map_err(|p| anyhow::anyhow!("non-utf8 path {}", p.display()))
}

pub fn safe_output_path_for_source(
    job_dir: &Path,
    source: &str,
    extension: &str,
) -> Result<Utf8PathBuf> {
    let stem = input_stem(source);
    let filename = format!("{}.{}", stem, extension.trim_start_matches('.'));
    let path = next_available_path(&job_dir.join(filename));
    Utf8PathBuf::from_path_buf(path).map_err(|p| anyhow::anyhow!("non-utf8 path {}", p.display()))
}

pub fn input_stem(source: &str) -> String {
    let clean = source
        .split(['?', '#'])
        .next()
        .unwrap_or(source)
        .trim_end_matches(['/', '\\']);
    let segment = clean
        .rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("output");
    let stem = segment
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(segment);
    slug(stem)
}

pub fn input_extension(source: &str, fallback: &str) -> String {
    let clean = source
        .split(['?', '#'])
        .next()
        .unwrap_or(source)
        .trim_end_matches(['/', '\\']);
    let segment = clean.rsplit(['/', '\\']).next().unwrap_or(clean);
    segment
        .rsplit_once('.')
        .map(|(_, ext)| ext)
        .filter(|ext| !ext.is_empty())
        .unwrap_or(fallback)
        .trim_start_matches('.')
        .to_ascii_lowercase()
}

pub fn next_available_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = path.extension().and_then(|s| s.to_str());
    for index in 2.. {
        let name = match ext {
            Some(ext) => format!("{stem}-{index}.{ext}"),
            None => format!("{stem}-{index}"),
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

pub fn slug(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string().if_empty("job")
}

pub fn parse_size_bytes(value: &str) -> Result<u64> {
    let trimmed = value.trim().to_ascii_lowercase();
    let split_at = trimmed
        .find(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .unwrap_or(trimmed.len());
    let (number, unit) = trimmed.split_at(split_at);
    let n: f64 = number
        .parse()
        .with_context(|| format!("invalid size `{value}`"))?;
    let multiplier = match unit.trim() {
        "" | "b" => 1.0,
        "k" | "kb" => 1_000.0,
        "m" | "mb" => 1_000_000.0,
        "g" | "gb" => 1_000_000_000.0,
        "ki" | "kib" => 1024.0,
        "mi" | "mib" => 1024.0 * 1024.0,
        "gi" | "gib" => 1024.0 * 1024.0 * 1024.0,
        other => bail!("unknown size unit `{other}`"),
    };
    Ok((n * multiplier) as u64)
}

pub fn dir_size(paths: &[Utf8PathBuf]) -> u64 {
    paths
        .iter()
        .filter_map(|path| fs::metadata(path).ok().map(|meta| meta.len()))
        .sum()
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub fn format_duration(seconds: Option<f64>) -> String {
    let Some(seconds) = seconds else {
        return "unknown".to_string();
    };
    let d = Duration::from_secs_f64(seconds.max(0.0));
    let total = d.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        (total % 3600) / 60,
        total % 60
    )
}

pub fn shell_quote(arg: &str) -> String {
    if arg
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./:=\\ ".contains(c))
        && !arg.contains(' ')
    {
        arg.to_string()
    } else {
        format!("\"{}\"", arg.replace('"', "\\\""))
    }
}

trait IfEmpty {
    fn if_empty(self, fallback: &str) -> String;
}

impl IfEmpty for String {
    fn if_empty(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{input_extension, input_stem};

    #[test]
    fn url_sources_have_stable_names() {
        assert_eq!(
            input_stem("https://cdn.example.com/media/Launch Clip.mp4?token=abc"),
            "launch-clip"
        );
        assert_eq!(
            input_extension(
                "https://cdn.example.com/media/Launch Clip.mp4?token=abc",
                "bin"
            ),
            "mp4"
        );
        assert_eq!(input_stem("https://cdn.example.com/media/"), "media");
    }
}
