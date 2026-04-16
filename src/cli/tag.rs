use crate::client::diagv1::UpdateTagRequest;
use crate::error::TldError;
use crate::output;
use crate::workspace;
use clap::{Args, Subcommand};

#[derive(Args, Debug, Clone)]
pub struct TagArgs {
    #[command(subcommand)]
    pub command: TagCommands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum TagCommands {
    /// Create or update a tag in the organization (sets color and description)
    Create(TagCreateArgs),
}

#[derive(Args, Debug, Clone)]
pub struct TagCreateArgs {
    /// Tag name
    pub name: String,
    /// Hex color for the tag (e.g. #FF5733)
    #[arg(long, default_value = "#888888")]
    pub color: String,
    /// Optional description for the tag
    #[arg(long)]
    pub description: Option<String>,
}

pub async fn exec(args: TagArgs, _wdir: String) -> Result<(), TldError> {
    match args.command {
        TagCommands::Create(ref create_args) => exec_create(create_args).await,
    }
}

async fn exec_create(args: &TagCreateArgs) -> Result<(), TldError> {
    let cfg = workspace::load_config()?;

    if cfg.api_key.is_empty() {
        return Err(TldError::Generic(
            "Not authenticated. Run 'tld login' first.".to_string(),
        ));
    }

    let tag = args.name.trim().to_string();
    if tag.is_empty() {
        return Err(TldError::Generic("Tag name cannot be empty.".to_string()));
    }

    let mut client = crate::client::new_org_client(&cfg.server_url, &cfg.api_key).await?;

    let req = UpdateTagRequest {
        tag: tag.clone(),
        color: args.color.clone(),
        description: args.description.clone(),
    };

    client
        .update_tag(req)
        .await
        .map_err(|e| TldError::Generic(format!("Failed to create tag: {e}")))?;

    output::print_ok(&format!("Tag '{tag}' created/updated."));
    output::print_info("Run 'tld apply' to push element tag assignments to the server.");

    Ok(())
}
