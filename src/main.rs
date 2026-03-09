use clap::Parser;
use forge_cli::cli::{Cli, Commands};
use forge_cli::providers::{Role, ProviderFactory, MockProvider};
use forge_cli::providers::openrouter::OpenRouterProvider;
use forge_cli::providers::google::GoogleProvider;
use forge_cli::providers::grok::GrokProvider;
use forge_cli::providers::local::LocalProvider;
use forge_cli::session::{Session, SessionManager};
use forge_cli::context::ContextManager;
use forge_cli::orchestrator::Orchestrator;
use forge_cli::config::{Config, ApiConfig, LocalConfig};
use directories::ProjectDirs;
use std::env;
use tracing_subscriber::EnvFilter;
use inquire::{Text, Select};
use colored::Colorize;
use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let mut config = Config::load()?;

    let proj_dirs = ProjectDirs::from("com", "forge", "forge-cli")
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    let session_manager = SessionManager::new(proj_dirs.data_dir().join("sessions"))?;

    match &cli.command {
        Commands::Chat { prompt, provider, model, file, repo } => {
            // 1. Determine Provider
            let provider_name = match provider.clone().or(config.default_provider.clone()) {
                Some(p) => p,
                None => {
                    // If no default, but we have exactly one configured, use it.
                    let configured = get_configured_providers(&config);
                    if configured.len() == 1 {
                        let p = configured[0].clone();
                        info!(provider = %p, "Automatically selecting the only configured provider.");
                        p
                    } else {
                        println!("{} No default provider set.", "ℹ".blue());
                        let options = vec!["local", "openrouter", "google", "grok", "mock"];
                        let selection = Select::new("Which provider would you like to use?", options).prompt()?.to_string();
                        
                        // Save this as default for next time
                        config.default_provider = Some(selection.clone());
                        config.save()?;
                        selection
                    }
                }
            };

            // 2. Proactive Configuration Check
            if !is_provider_configured(&provider_name, &config) {
                println!("{} Provider '{}' requires setup.", "ℹ".blue(), provider_name.bold());
                configure_provider(&provider_name, &mut config).await?;
                
                // If we just configured it, make it the default
                config.default_provider = Some(provider_name.clone());
                config.save()?;
            }

            // 3. Initialize Factory
            let mut factory = ProviderFactory::new();
            register_providers(&mut factory, &config);
            factory.register("mock", Box::new(MockProvider));

            let mut session = Session::new("default".to_string(), provider_name.clone(), model.clone().or(config.default_model.clone()));
            
            let mut context_payload = String::new();
            if !file.is_empty() { context_payload.push_str(&ContextManager::read_files(file)?); }
            if *repo { context_payload.push_str(&ContextManager::get_repo_map(&env::current_dir()?)?); }

            if !context_payload.is_empty() {
                session.add_message(Role::System, format!("Context:\n{}\nUse <shell>cmd</shell> for tools.", context_payload));
            } else {
                session.add_message(Role::System, "Use <shell>cmd</shell> for tools.".to_string());
            }

            let mut orchestrator = Orchestrator::new(factory, provider_name, session, session_manager);
            orchestrator.run(prompt.clone()).await?;
        }
        Commands::Setup => {
            run_full_setup(&mut config).await?;
        }
        Commands::Config => {
            println!("Config path: {}", Config::config_path()?.display());
            println!("Default provider: {:?}", config.default_provider);
        }
        Commands::Sessions => {
            let sessions = session_manager.list_sessions()?;
            println!("{}:", "Saved Sessions".yellow().bold());
            for s in sessions {
                println!("  - {}", s);
            }
        }
    }

    Ok(())
}

fn is_provider_configured(name: &str, config: &Config) -> bool {
    let has_config = match name {
        "openrouter" => config.providers.openrouter.as_ref().map(|c| !c.api_key.trim().is_empty()).unwrap_or(false),
        "google" => config.providers.google.as_ref().map(|c| !c.api_key.trim().is_empty()).unwrap_or(false),
        "grok" => config.providers.grok.as_ref().map(|c| !c.api_key.trim().is_empty()).unwrap_or(false),
        "local" => config.providers.local.is_some(),
        "mock" => true,
        _ => false,
    };

    let has_env = match name {
        "openrouter" => env::var("OPENROUTER_API_KEY").map(|k| !k.trim().is_empty()).unwrap_or(false),
        "google" => env::var("GOOGLE_API_KEY").map(|k| !k.trim().is_empty()).unwrap_or(false),
        "grok" => env::var("GROK_API_KEY").map(|k| !k.trim().is_empty()).unwrap_or(false),
        _ => false,
    };

    has_config || has_env
}

fn get_configured_providers(config: &Config) -> Vec<String> {
    let mut providers = Vec::new();
    for name in &["openrouter", "google", "grok", "local"] {
        if is_provider_configured(name, config) {
            providers.push(name.to_string());
        }
    }
    providers
}

async fn configure_provider(name: &str, config: &mut Config) -> Result<()> {
    match name {
        "openrouter" => {
            let key = Text::new("Enter OpenRouter API Key:").prompt()?;
            let trimmed = key.trim().to_string();
            if !trimmed.is_empty() { config.providers.openrouter = Some(ApiConfig { api_key: trimmed }); }
        }
        "google" => {
            let key = Text::new("Enter Google Gemini API Key:").prompt()?;
            let trimmed = key.trim().to_string();
            if !trimmed.is_empty() { config.providers.google = Some(ApiConfig { api_key: trimmed }); }
        }
        "grok" => {
            let key = Text::new("Enter Grok (xAI) API Key:").prompt()?;
            let trimmed = key.trim().to_string();
            if !trimmed.is_empty() { config.providers.grok = Some(ApiConfig { api_key: trimmed }); }
        }
        "local" => {
            let url = Text::new("Enter Local Provider URL (e.g. http://localhost:11434):")
                .with_default("http://localhost:11434")
                .prompt()?;
            let trimmed = url.trim().to_string();
            if !trimmed.is_empty() { config.providers.local = Some(LocalConfig { base_url: trimmed }); }
        }
        _ => {}
    }
    println!("{} Configuration updated.", "✓".green());
    Ok(())
}

fn register_providers(factory: &mut ProviderFactory, config: &Config) {
    // OpenRouter
    let or_key = config.providers.openrouter.as_ref()
        .map(|c| c.api_key.trim().to_string())
        .filter(|k| !k.is_empty())
        .or_else(|| env::var("OPENROUTER_API_KEY").ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()));
    
    if let Some(key) = or_key {
        factory.register("openrouter", Box::new(OpenRouterProvider::new(key)));
    }

    // Google
    let g_key = config.providers.google.as_ref()
        .map(|c| c.api_key.trim().to_string())
        .filter(|k| !k.is_empty())
        .or_else(|| env::var("GOOGLE_API_KEY").ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()));
    
    if let Some(key) = g_key {
        factory.register("google", Box::new(GoogleProvider::new(key)));
    }

    // Grok
    let x_key = config.providers.grok.as_ref()
        .map(|c| c.api_key.trim().to_string())
        .filter(|k| !k.is_empty())
        .or_else(|| env::var("GROK_API_KEY").ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()));
    
    if let Some(key) = x_key {
        factory.register("grok", Box::new(GrokProvider::new(key)));
    }

    let local_url = config.providers.local.as_ref().map(|l| l.base_url.trim().to_string()).unwrap_or_else(|| "http://localhost:11434".to_string());
    factory.register("local", Box::new(LocalProvider::new(local_url)));
}

async fn run_full_setup(config: &mut Config) -> Result<()> {
    println!("{}", "Forge CLI Full Setup Wizard".yellow().bold());
    let options = vec!["OpenRouter", "Google Gemini", "Grok (xAI)", "Local (Ollama/vLLM)", "Set Default Provider", "Finish"];
    
    loop {
        let selection = Select::new("What would you like to configure?", options.clone()).prompt()?;
        match selection {
            "OpenRouter" => configure_provider("openrouter", config).await?,
            "Google Gemini" => configure_provider("google", config).await?,
            "Grok (xAI)" => configure_provider("grok", config).await?,
            "Local (Ollama/vLLM)" => configure_provider("local", config).await?,
            "Set Default Provider" => {
                let p = Select::new("Default Provider:", vec!["openrouter", "google", "grok", "local", "mock"]).prompt()?;
                config.default_provider = Some(p.to_string());
            }
            _ => break,
        }
        config.save()?;
    }
    Ok(())
}
