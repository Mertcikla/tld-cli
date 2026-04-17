#![expect(clippy::print_stdout, clippy::print_stderr, clippy::expect_used)]
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use tabled::settings::{Color, Style as TStyle, object::Rows};
use tabled::{Table, Tabled};

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

// ── Spinner helpers ────────────────────────────────────────────────────────────

const SPINNER_CHARS: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";
const PROGRESS_CHARS: &str = "=>-";

/// Create a spinner with an initial message. Caller drives it.
pub fn new_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .expect("should have valid progress template")
            .tick_chars(SPINNER_CHARS),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Create a determinate progress bar with an initial message.
pub fn new_progress_bar(len: u64, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::with_template("{bar:24.cyan/blue} {pos:>4}/{len:4} {percent:>3}% {msg}")
            .expect("should have valid progress template")
            .progress_chars(PROGRESS_CHARS),
    );
    pb.set_message(msg.to_string());
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
    println!("{table}");
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
