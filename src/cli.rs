use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "forge")]
#[command(author, version, about = "A truly provider-agnostic, local-first orchestrator CLI.", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start a chat with the selected provider
    Chat {
        /// The message to send
        prompt: Option<String>,
        /// The provider to use (e.g., openrouter, google, local, mock)
        #[arg(short, long)]
        provider: Option<String>,
        /// The model to use
        #[arg(short, long)]
        model: Option<String>,
        /// Attach one or more files as context
        #[arg(short, long)]
        file: Vec<String>,
        /// Scan the current directory and provide a file map as context
        #[arg(short, long)]
        repo: bool,
    },
    /// List all saved chat sessions
    Sessions,
    /// Configure Forge CLI
    Config,
    /// Setup Wizard for local models and API keys
    Setup,
}
