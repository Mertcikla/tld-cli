use clap::Args;

pub const DISPLAY_VERSION: &str = env!("TLD_BUILD_VERSION");

// Version has no args — handled directly in main.
#[derive(Args, Debug)]
pub struct VersionArgs {}
