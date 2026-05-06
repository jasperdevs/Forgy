use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use walkdir::WalkDir;

const MEDIA_EXTS: &[&str] = &[
    "mp4", "mov", "mkv", "webm", "avi", "mp3", "wav", "m4a", "flac", "png", "jpg", "jpeg", "webp",
];

pub fn collect_media(folder: &str, recursive: bool) -> Result<Vec<Utf8PathBuf>> {
    let mut files = Vec::new();
    let walker = if recursive {
        WalkDir::new(folder)
    } else {
        WalkDir::new(folder).max_depth(1)
    };
    for entry in walker {
        let entry = entry.with_context(|| format!("failed to read {folder}"))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if MEDIA_EXTS
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(ext))
        {
            files.push(
                Utf8PathBuf::from_path_buf(path.to_path_buf())
                    .map_err(|p| anyhow::anyhow!("non-utf8 media path {}", p.display()))?,
            );
        }
    }
    files.sort();
    Ok(files)
}
