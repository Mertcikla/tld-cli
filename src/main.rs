mod analyzer;
mod cli;
mod client;
mod error;
mod output;
mod planner;
mod workspace;

use crate::cli::{Cli, Commands};
use crate::error::TldError;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), TldError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(ref args) => {
            let wdir = cli.workspace_dir();
            cli::init::exec(args.clone(), wdir).await?;
        }
        Commands::Login(ref args) => {
            let wdir = cli.workspace_dir();
            cli::login::exec(args.clone(), wdir).await?;
        }
        Commands::Apply(ref args) => {
            let wdir = cli.workspace_dir();
            cli::apply::exec(args.clone(), wdir).await?;
        }
        Commands::Plan(ref args) => {
            let wdir = cli.workspace_dir();
            cli::plan::exec(args.clone(), wdir).await?;
        }
        Commands::Add(ref args) => {
            let wdir = cli.workspace_dir();
            cli::add::exec(args.clone(), wdir).await?;
        }
        Commands::Connect(ref args) => {
            let wdir = cli.workspace_dir();
            cli::connect::exec(args.clone(), wdir).await?;
        }
        Commands::Remove(ref args) => {
            let wdir = cli.workspace_dir();
            cli::remove::exec(args.clone(), wdir).await?;
        }
        Commands::Update(ref args) => {
            let wdir = cli.workspace_dir();
            cli::update::exec(args.clone(), wdir).await?;
        }
        Commands::Validate(ref args) => {
            let wdir = cli.workspace_dir();
            cli::validate::exec(args.clone(), wdir).await?;
        }
        Commands::Export(ref args) => {
            let wdir = cli.workspace_dir();
            cli::export::exec(args.clone(), wdir).await?;
        }
        Commands::Pull(ref args) => {
            let wdir = cli.workspace_dir();
            cli::pull::exec(args.clone(), wdir).await?;
        }
        Commands::Status(ref args) => {
            let wdir = cli.workspace_dir();
            cli::status::exec(args.clone(), wdir).await?;
        }
        Commands::Views(ref args) => {
            let wdir = cli.workspace_dir();
            cli::views::exec(args.clone(), wdir).await?;
        }
        Commands::Diff(ref args) => {
            let wdir = cli.workspace_dir();
            cli::diff::exec(args.clone(), wdir).await?;
        }
        Commands::Check(ref args) => {
            let wdir = cli.workspace_dir();
            cli::check::exec(args.clone(), wdir).await?;
        }
        Commands::Analyze(ref args) => {
            let wdir = cli.workspace_dir();
            cli::analyze::exec(args.clone(), wdir).await?;
        }
        _ => {
            output::print_warn(&format!("Command {:?} is not yet implemented", cli.command));
        }
    }

    Ok(())
}
