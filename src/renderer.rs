use std::io::Write;
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

use crate::parser::{count_actions, ChangeCounts, Format, ResourceChange};

#[derive(Clone, Copy)]
pub struct LevelWriter;

pub enum OutputWriter {
    Stdout(std::io::Stdout),
    Stderr(std::io::Stderr),
}

impl<'writer> MakeWriter<'writer> for LevelWriter {
    type Writer = OutputWriter;

    fn make_writer(&'writer self) -> Self::Writer {
        OutputWriter::Stderr(std::io::stderr())
    }

    fn make_writer_for(&'writer self, meta: &tracing::Metadata<'_>) -> Self::Writer {
        match *meta.level() {
            Level::INFO => OutputWriter::Stdout(std::io::stdout()),
            _ => OutputWriter::Stderr(std::io::stderr()),
        }
    }
}

impl std::io::Write for OutputWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Stdout(writer) => writer.write(buf),
            Self::Stderr(writer) => writer.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Stdout(writer) => writer.flush(),
            Self::Stderr(writer) => writer.flush(),
        }
    }
}

pub fn summary_action_symbols(no_emoji: bool) -> (&'static str, &'static str, &'static str) {
    if no_emoji {
        ("+", "~", "-")
    } else {
        ("➕", "🔄", "➖")
    }
}

pub fn render_summary_line(counts: &ChangeCounts, no_emoji: bool) -> String {
    let (create_sym, update_sym, delete_sym) = summary_action_symbols(no_emoji);
    format!(
        "Summary:\n {create_sym} {} to create\n {update_sym} {} to update\n {delete_sym} {} to delete\n",
        counts.create, counts.update, counts.delete
    )
}

pub fn render_github_step_summary(
    display_path: &std::path::Path,
    resource_changes: &[ResourceChange],
    counts: &ChangeCounts,
    no_emoji: bool,
) -> String {
    use std::fmt::Write;

    let (create_sym, update_sym, delete_sym) = summary_action_symbols(no_emoji);
    let mut output = String::new();
    writeln!(output, "## Terraform plan summary").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "**Plan:** `{}`", display_path.display()).unwrap();
    writeln!(output).unwrap();
    writeln!(output, "| | Count |").unwrap();
    writeln!(output, "| --- | ---: |").unwrap();
    writeln!(output, "| {create_sym} Create | {} |", counts.create).unwrap();
    writeln!(output, "| {update_sym} Update | {} |", counts.update).unwrap();
    writeln!(output, "| {delete_sym} Delete | {} |", counts.delete).unwrap();

    if !resource_changes.is_empty() {
        writeln!(output).unwrap();
        writeln!(output, "### Resource changes").unwrap();
        writeln!(output).unwrap();
        writeln!(output, "| Action | Type | Name |").unwrap();
        writeln!(output, "| --- | --- | --- |").unwrap();
        for change in resource_changes {
            writeln!(
                output,
                "| {} | {} | {} |",
                change.action, change.resource_type, change.resource_name
            )
            .unwrap();
        }
    }

    output
}

pub fn append_github_step_summary(
    summary_path: &str,
    display_path: &std::path::Path,
    resource_changes: &[ResourceChange],
    counts: &ChangeCounts,
    no_emoji: bool,
) -> std::io::Result<()> {
    let markdown = render_github_step_summary(display_path, resource_changes, counts, no_emoji);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(summary_path)?;
    if file.metadata()?.len() > 0 {
        writeln!(file)?;
    }
    write!(file, "{markdown}")?;
    if !markdown.ends_with('\n') {
        writeln!(file)?;
    }
    Ok(())
}

pub fn should_write_github_summary(settings: &crate::cli::AppSettings) -> bool {
    std::env::var_os("GITHUB_STEP_SUMMARY").is_some() || settings.github_summary
}

pub fn write_github_summary_if_enabled(
    settings: &crate::cli::AppSettings,
    display_path: &std::path::Path,
    resource_changes: &[ResourceChange],
) {
    if !should_write_github_summary(settings) {
        return;
    }

    let Some(summary_path) = std::env::var_os("GITHUB_STEP_SUMMARY") else {
        if settings.github_summary {
            tracing::warn!(
                "--github-summary was set but GITHUB_STEP_SUMMARY is not set; skipping summary"
            );
        }
        return;
    };

    let summary_path = summary_path.to_string_lossy();
    let counts = count_actions(resource_changes);
    if let Err(error) = append_github_step_summary(
        &summary_path,
        display_path,
        resource_changes,
        &counts,
        settings.no_emoji,
    ) {
        tracing::warn!("Failed to write GitHub Actions summary: {error}");
    }
}

pub fn render_changes(
    resource_changes: &[ResourceChange],
    abs_path: &std::path::Path,
    format: &Format,
    no_emoji: bool,
    quiet: bool,
    no_header: bool,
) -> String {
    let counts = count_actions(resource_changes);
    match format {
        Format::Text => render_text(resource_changes, abs_path, no_emoji, quiet, &counts),
        Format::Json => render_json(resource_changes),
        Format::Csv => render_csv(resource_changes, no_header),
        Format::Table => render_table(resource_changes, abs_path, no_emoji, quiet, &counts),
    }
}

pub fn render_text(
    resource_changes: &[ResourceChange],
    abs_path: &std::path::Path,
    no_emoji: bool,
    quiet: bool,
    counts: &ChangeCounts,
) -> String {
    let mut output = String::new();
    if resource_changes.is_empty() {
        let prefix = if no_emoji { "" } else { "✅ " };
        output.push_str(&format!(
            "{}No resource changes detected in '{}'.\n",
            prefix,
            abs_path.display()
        ));
        if !quiet {
            output.push_str(&render_summary_line(counts, no_emoji));
        }
        return output;
    }

    let prefix = if no_emoji { "" } else { "📊 " };
    output.push_str(&format!(
        "{}Planned changes in '{}':\n",
        prefix,
        abs_path.display()
    ));
    for change in resource_changes {
        let symbol = if no_emoji {
            match change.action.as_str() {
                "create" => "+ ",
                "update" => "~ ",
                "delete" => "- ",
                "read" => "? ",
                _ => "* ",
            }
        } else {
            match change.action.as_str() {
                "create" => "➕ ",
                "update" => "🔄 ",
                "delete" => "➖ ",
                "read" => "📖 ",
                _ => "• ",
            }
        };
        output.push_str(&format!(
            "{}{} {} ({})\n",
            symbol, change.resource_type, change.resource_name, change.action
        ));
    }
    if !quiet {
        output.push_str(&render_summary_line(counts, no_emoji));
    }
    output
}

pub fn render_json(resource_changes: &[ResourceChange]) -> String {
    format!(
        "{}\n",
        serde_json::to_string_pretty(resource_changes).expect("resource changes serialize to JSON")
    )
}

pub fn render_csv(resource_changes: &[ResourceChange], no_header: bool) -> String {
    let mut output = String::new();
    if !no_header {
        output.push_str("resource_type,resource_name,action\n");
    }
    for change in resource_changes {
        output.push_str(&format!(
            "{},{},{}\n",
            csv_escape(&change.resource_type),
            csv_escape(&change.resource_name),
            csv_escape(&change.action)
        ));
    }
    output
}

pub fn render_table(
    resource_changes: &[ResourceChange],
    abs_path: &std::path::Path,
    no_emoji: bool,
    quiet: bool,
    counts: &ChangeCounts,
) -> String {
    if resource_changes.is_empty() {
        let mut output = format!(
            "No resource changes detected in '{}'.\n",
            abs_path.display()
        );
        if !quiet {
            output.push_str(&render_summary_line(counts, no_emoji));
        }
        return output;
    }

    let type_width = resource_changes
        .iter()
        .map(|change| change.resource_type.len())
        .chain(["Resource Type".len()])
        .max()
        .unwrap_or("Resource Type".len());
    let name_width = resource_changes
        .iter()
        .map(|change| change.resource_name.len())
        .chain(["Resource Name".len()])
        .max()
        .unwrap_or("Resource Name".len());
    let action_width = resource_changes
        .iter()
        .map(|change| change.action.len())
        .chain(["Action".len()])
        .max()
        .unwrap_or("Action".len());

    let mut output = format!("Planned changes in '{}':\n", abs_path.display());
    output.push_str(&format!(
        "{:type_width$}  {:name_width$}  {:action_width$}\n",
        "Resource Type", "Resource Name", "Action"
    ));
    output.push_str(&format!(
        "{:-<type_width$}  {:-<name_width$}  {:-<action_width$}\n",
        "", "", ""
    ));
    for change in resource_changes {
        output.push_str(&format!(
            "{:type_width$}  {:name_width$}  {:action_width$}\n",
            change.resource_type, change.resource_name, change.action
        ));
    }
    if !quiet {
        output.push_str(&render_summary_line(counts, no_emoji));
    }
    output
}

pub fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn write_rendered_output(
    output_file: Option<&std::path::Path>,
    rendered: &str,
) -> Result<(), String> {
    if let Some(path) = output_file {
        std::fs::write(path, rendered).map_err(|error| {
            format!(
                "Failed to write rendered output to '{}': {error}",
                path.display()
            )
        })?;
    } else {
        tracing::info!("{}", rendered.trim_end());
    }
    Ok(())
}
