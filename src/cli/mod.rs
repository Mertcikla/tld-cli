pub mod add;
/// CLI command definitions using `clap`.
/// Mirrors the Go `cmd/` package structure with identical command names.
pub mod analyze;
pub mod apply;
pub mod check;
pub mod connect;
pub mod diff;
pub mod export;
pub mod init;
pub mod login;
pub mod plan;
pub mod pull;
pub mod remove;
pub mod status;
pub mod tag;
pub mod update;
pub mod validate;
pub mod version;
pub mod views;

use clap::{Parser, Subcommand};

// ── Root CLI struct ────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "tld",
    version = env!("CARGO_PKG_VERSION"),
    about = "tld -- tlDiagram CLI",
    long_about = "tld manages software architecture diagrams as code.\n\n\
        Define your architecture in YAML, preview changes with 'tld plan',\n\
        and apply them atomically with 'tld apply'.",
    propagate_version = true,
)]
pub struct Cli {
    /// Workspace directory (default: .tld if present, else .)
    #[arg(short = 'w', long, global = true)]
    pub workspace: Option<String>,

    /// Output format: text or json
    #[arg(long, global = true, default_value = "text")]
    pub format: String,

    /// Compact JSON output (no whitespace)
    #[arg(long, global = true, default_value = "false")]
    pub compact: bool,

    /// Enable verbose output
    #[arg(short = 'v', long, global = true, default_value = "false")]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn workspace_dir(&self) -> String {
        crate::workspace::resolve_workspace_dir(self.workspace.as_deref())
    }
}

// ── Subcommand variants ───────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum Commands {
    // ── CRUD actions on resources ─────────────────────────────────────────────
    /// Add or update an element in elements.yaml
    Add(add::AddArgs),

    /// Add a connector between two elements
    Connect(connect::ConnectArgs),

    /// Remove workspace resources
    Remove(remove::RemoveArgs),

    /// Update a workspace resource field
    Update(update::UpdateArgs),

    // ── Secondary actions ─────────────────────────────────────────────────────
    /// Initialize a new tld workspace
    Init(init::InitArgs),

    /// Authenticate the CLI with a tlDiagram server
    Login(login::LoginArgs),

    /// Validate workspace YAML against schema rules
    Validate(validate::ValidateArgs),

    /// Show what would be applied (dry-run plan)
    Plan(plan::PlanArgs),

    /// Push workspace state to tlDiagram
    Apply(apply::ApplyArgs),

    /// Export workspace to a local snapshot
    Export(export::ExportArgs),

    /// Pull remote state into the local workspace
    Pull(pull::PullArgs),

    /// Show workspace sync status
    Status(status::StatusArgs),

    /// List views in the workspace
    Views(views::ViewsArgs),

    /// Diff the current workspace against the last known remote state
    Diff(diff::DiffArgs),

    /// Extract symbols from source files and upsert as workspace elements
    Analyze(analyze::AnalyzeArgs),

    /// Check workspace for architectural issues
    Check(check::CheckArgs),

    /// Manage tags (colors and descriptions) in the organization
    Tag(tag::TagArgs),

    /// Print the version number of tld
    Version,
}
