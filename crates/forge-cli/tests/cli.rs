use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn forge() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn have(tool: &str) -> bool {
    Command::new(tool).arg("-version").output().is_ok()
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn make_sample(dir: &Path) -> PathBuf {
    let sample = dir.join("sample.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=160x90:rate=12",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100",
            "-t",
            "1",
            "-pix_fmt",
            "yuv420p",
            sample.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    sample
}

#[test]
fn help_snapshot_mentions_core_commands() {
    let output = Command::new(forge()).arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("terminal-native media toolkit"));
    assert!(stdout.contains("Convert media to another common format"));
    assert!(stdout.contains("Wrap yt-dlp with Forgy output folders"));
    assert!(stdout.contains("inspect"));
    assert!(stdout.contains("doctor"));
    assert!(stdout.contains("youtube"));
}

#[test]
fn transcribe_and_notes_write_reports() {
    if !have("ffmpeg") {
        eprintln!("skipping media integration test because ffmpeg is unavailable");
        return;
    }
    let dir = temp_dir("text-workflows");
    let sample = dir.join("sample.wav");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100",
            "-t",
            "1",
            sample.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let transcribe_root = dir.join("transcribe-out");
    let transcribe = Command::new(forge())
        .args([
            "--output",
            transcribe_root.to_str().unwrap(),
            "transcribe",
            sample.to_str().unwrap(),
            "--srt",
        ])
        .output()
        .unwrap();
    assert!(transcribe.status.success());
    assert!(
        transcribe_root
            .join("transcribe")
            .join("forge-report.json")
            .exists()
    );

    let notes_root = dir.join("notes-out");
    let notes = Command::new(forge())
        .args([
            "--output",
            notes_root.to_str().unwrap(),
            "notes",
            sample.to_str().unwrap(),
            "--summary",
        ])
        .output()
        .unwrap();
    assert!(notes.status.success());
    assert!(notes_root.join("notes").join("forge-report.md").exists());
}

#[test]
fn preset_snapshot_lists_creator_presets() {
    let output = Command::new(forge())
        .args(["preset", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tiktok"));
    assert!(stdout.contains("youtube-short"));
    assert!(stdout.contains("podcast"));
}

#[test]
fn inspect_and_convert_tiny_fixture() {
    if !have("ffmpeg") || !have("ffprobe") {
        eprintln!("skipping media integration test because ffmpeg/ffprobe is unavailable");
        return;
    }
    let dir = temp_dir("inspect-and-convert");
    let sample = make_sample(&dir);
    let output_root = dir.join("out");

    let inspect = Command::new(forge())
        .args(["--json", "inspect", sample.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(inspect.status.success());
    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(stdout.contains("\"resolution\""));
    assert!(stdout.contains("160x90"));

    let convert = Command::new(forge())
        .args([
            "--output",
            output_root.to_str().unwrap(),
            "convert",
            sample.to_str().unwrap(),
            "--to",
            "mp3",
        ])
        .output()
        .unwrap();
    assert!(
        convert.status.success(),
        "{}",
        String::from_utf8_lossy(&convert.stderr)
    );
    assert!(
        fs::read_dir(&output_root).unwrap().any(|entry| entry
            .unwrap()
            .path()
            .join("forge-report.json")
            .exists())
    );
}
