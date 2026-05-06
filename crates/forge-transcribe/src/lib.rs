use std::process::Command;

use anyhow::{Context, Result, bail};
use camino::Utf8Path;
use forge_core::CommandSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub text: String,
    pub srt: Option<String>,
    pub provider: String,
}

pub trait TranscriptionProvider {
    fn name(&self) -> &'static str;
    fn available(&self) -> bool;
    fn transcribe(&self, input: &Utf8Path, srt: bool) -> Result<Transcript>;
}

pub struct MockProvider;

impl TranscriptionProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn available(&self) -> bool {
        true
    }

    fn transcribe(&self, input: &Utf8Path, srt: bool) -> Result<Transcript> {
        Ok(Transcript {
            text: format!("Mock transcript for {input}"),
            srt: srt.then(|| "1\n00:00:00,000 --> 00:00:01,000\nMock transcript.\n".to_string()),
            provider: self.name().to_string(),
        })
    }
}

pub struct ExternalCommandProvider {
    pub command: String,
}

impl TranscriptionProvider for ExternalCommandProvider {
    fn name(&self) -> &'static str {
        "external-command"
    }

    fn available(&self) -> bool {
        Command::new(&self.command).arg("--help").output().is_ok()
    }

    fn transcribe(&self, input: &Utf8Path, _srt: bool) -> Result<Transcript> {
        let output = CommandSpec::new(&self.command)
            .arg(input.as_str())
            .run()
            .with_context(|| format!("external transcription command failed for {input}"))?;
        Ok(Transcript {
            text: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            srt: None,
            provider: self.name().to_string(),
        })
    }
}

pub struct WhisperCppProvider;

impl TranscriptionProvider for WhisperCppProvider {
    fn name(&self) -> &'static str {
        "whisper.cpp"
    }

    fn available(&self) -> bool {
        Command::new("whisper-cli").arg("--help").output().is_ok()
            || Command::new("main").arg("--help").output().is_ok()
    }

    fn transcribe(&self, _input: &Utf8Path, _srt: bool) -> Result<Transcript> {
        bail!(
            "whisper.cpp provider is detected as an architecture placeholder; configure an external command provider for now"
        )
    }
}
