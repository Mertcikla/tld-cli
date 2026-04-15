pub mod json;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use tabled::{Table, Tabled};
use tabled::settings::{Style as TStyle, object::Rows, Color};

// ── Status helpers ─────────────────────────────────────────────────────────────

/// Print a success line:  ✓  <message>
pub fn print_ok(msg: &str) {
    eprintln!("  {}  {}", style("✓").green().bold(), msg);
}

/// Print an error line:  ✗  <message>
pub fn print_err(msg: &str) {
    eprintln!("  {}  {}", style("✗").red().bold(), msg);
}

/// Print a warning line:  ⚠  <message>
pub fn print_warn(msg: &str) {
    eprintln!("  {}  {}", style("⚠").yellow().bold(), msg);
}

/// Print an informational line:  ·  <message>
pub fn print_info(msg: &str) {
    eprintln!("  {}  {}", style("·").dim(), msg);
}

/// Print a section header with a decorative line.
pub fn print_header(title: &str) {
    let line = "─".repeat(60);
    eprintln!("\n{}", style(title).bold().underlined());
    eprintln!("{}", style(&line).dim());
}

// ── Spinner helpers ────────────────────────────────────────────────────────────

const SPINNER_CHARS: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";

/// Create a spinner with an initial message. Caller drives it.
pub fn new_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars(SPINNER_CHARS),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Create a bounded progress bar with a description.
pub fn new_progress_bar(total: u64, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan} {msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})"
        )
        .unwrap()
        .tick_chars(SPINNER_CHARS)
        .progress_chars("██░"),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

// ── Table helpers ──────────────────────────────────────────────────────────────

/// Render a Vec of `Tabled` items as a styled table to stdout.
pub fn print_table<T: Tabled>(rows: Vec<T>) {
    if rows.is_empty() {
        println!("  (no items)");
        return;
    }
    let mut table = Table::new(rows);
    table
        .with(TStyle::modern())
        .modify(Rows::first(), Color::BOLD);
    println!("{}", table);
}

/// Key-value summary table (two-column layout).
#[derive(Tabled)]
pub struct KvRow {
    #[tabled(rename = "Key")]
    pub key: String,
    #[tabled(rename = "Value")]
    pub value: String,
}

pub fn print_kv_table(pairs: Vec<(&str, String)>) {
    let rows: Vec<KvRow> = pairs
        .into_iter()
        .map(|(k, v)| KvRow {
            key: k.to_string(),
            value: v,
        })
        .collect();
    print_table(rows);
}

// ── JSON helpers ───────────────────────────────────────────────────────────────

/// Print a value as pretty-printed JSON to stdout.
pub fn print_json<T: serde::Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
}

/// Output format selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

impl OutputFormat {
    pub fn is_json(&self) -> bool {
        matches!(self, OutputFormat::Json)
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "text" | "" => Ok(OutputFormat::Text),
            other => Err(format!("unknown format: {}", other)),
        }
    }
}
