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
#[expect(clippy::print_stdout)]
async fn main() -> Result<(), TldError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(ref args) => {
            let wdir = cli.workspace_dir();
            cli::init::exec(args.clone(), wdir)?;
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
            cli::add::exec(args.clone(), wdir)?;
        }
        Commands::Connect(ref args) => {
            let wdir = cli.workspace_dir();
            cli::connect::exec(args.clone(), wdir)?;
        }
        Commands::Remove(ref args) => {
            let wdir = cli.workspace_dir();
            cli::remove::exec(args.clone(), wdir)?;
        }
        Commands::Update(ref args) => {
            let wdir = cli.workspace_dir();
            cli::update::exec(args.clone(), wdir)?;
        }
        Commands::Validate(ref args) => {
            let wdir = cli.workspace_dir();
            cli::validate::exec(args.clone(), wdir)?;
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
            cli::status::exec(args.clone(), wdir)?;
        }
        Commands::Views(ref args) => {
            let wdir = cli.workspace_dir();
            cli::views::exec(args.clone(), wdir)?;
        }
        Commands::Diff(ref args) => {
            let wdir = cli.workspace_dir();
            cli::diff::exec(args.clone(), wdir).await?;
        }
        Commands::Check(ref args) => {
            let wdir = cli.workspace_dir();
            cli::check::exec(args.clone(), wdir)?;
        }
        Commands::Analyze(ref args) => {
            let wdir = cli.workspace_dir();
            cli::analyze::exec(args.clone(), wdir).await?;
        }
        Commands::Tag(ref args) => {
            let wdir = cli.workspace_dir();
            cli::tag::exec(args.clone(), wdir).await?;
        }
        Commands::Version => {
            println!("tld {}", cli::version::DISPLAY_VERSION);
        }
    }

    Ok(())
}
