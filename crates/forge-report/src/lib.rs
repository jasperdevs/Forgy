use std::fs;

use anyhow::{Context, Result};
use forge_core::{ForgeReport, human_bytes};

pub fn write_report(report: &ForgeReport) -> Result<()> {
    let Some(job_dir) = report.output_files.first().and_then(|p| p.parent()) else {
        return Ok(());
    };
    let json = job_dir.join("forge-report.json");
    let md = job_dir.join("forge-report.md");
    fs::write(&json, serde_json::to_string_pretty(report)?)
        .with_context(|| format!("failed to write {json}"))?;
    fs::write(&md, render_markdown(report)).with_context(|| format!("failed to write {md}"))?;
    Ok(())
}

pub fn render_markdown(report: &ForgeReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Forge Report: {}\n\n", report.job_name));
    out.push_str(&format!("- Duration: {} ms\n", report.duration_ms));
    out.push_str(&format!(
        "- Size: {} -> {}\n",
        human_bytes(report.size_before_bytes),
        human_bytes(report.size_after_bytes)
    ));
    out.push_str("\n## Inputs\n");
    for input in &report.input_files {
        out.push_str(&format!("- `{input}`\n"));
    }
    out.push_str("\n## Outputs\n");
    for output in &report.output_files {
        out.push_str(&format!("- `{output}`\n"));
    }
    out.push_str("\n## Operations\n");
    for op in &report.operations_performed {
        out.push_str(&format!("- {op}\n"));
    }
    out.push_str("\n## Commands\n");
    for command in &report.commands_run {
        out.push_str(&format!("- `{command}`\n"));
    }
    if !report.quality_settings.is_empty() {
        out.push_str("\n## Quality Settings\n");
        for setting in &report.quality_settings {
            out.push_str(&format!("- {setting}\n"));
        }
    }
    if !report.warnings.is_empty() {
        out.push_str("\n## Warnings\n");
        for warning in &report.warnings {
            out.push_str(&format!("- {warning}\n"));
        }
    }
    out
}
