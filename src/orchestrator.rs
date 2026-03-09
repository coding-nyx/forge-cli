use crate::providers::{ChatRequest, Role, ProviderFactory};
use crate::session::{Session, SessionManager};
use crate::tools::{Tool, ShellTool};
use crate::ui::UI;
use crate::ui::parser::{ResponseParser, ResponsePart};
use futures_util::StreamExt;
use std::io::{self, Write};
use inquire::{Text, Confirm};
use colored::Colorize;
use anyhow::Result;
use tracing::error;

pub struct Orchestrator {
    factory: ProviderFactory,
    provider_name: String,
    session: Session,
    session_manager: SessionManager,
    tools: Vec<Box<dyn Tool>>,
}

impl Orchestrator {
    pub fn new(
        factory: ProviderFactory,
        provider_name: String,
        session: Session,
        session_manager: SessionManager,
    ) -> Self {
        Self {
            factory,
            provider_name,
            session,
            session_manager,
            tools: vec![Box::new(ShellTool)],
        }
    }

    pub async fn run(&mut self, initial_prompt: Option<String>) -> Result<()> {
        let mut current_prompt = initial_prompt.unwrap_or_default();
        let mut tool_call_count = 0;
        const MAX_TOOL_CALLS: usize = 5;

        loop {
            if current_prompt.is_empty() {
                tool_call_count = 0;
                match Text::new("You:").prompt() {
                    Ok(input) => {
                        let trimmed = input.trim();
                        if trimmed == "exit" || trimmed == "quit" { break; }
                        if trimmed.starts_with('/') {
                            self.handle_slash_command(trimmed).await?;
                            continue;
                        }
                        current_prompt = input;
                    }
                    Err(_) => break,
                }
            }

            self.session.add_message(Role::User, current_prompt.clone());
            current_prompt = String::new();

            let provider = self.factory.get(&self.provider_name)
                .ok_or_else(|| anyhow::anyhow!("Active provider '{}' not found", self.provider_name))?;

            let spinner = UI::create_spinner(&format!("Thinking ({})", self.provider_name));
            
            let request = ChatRequest {
                messages: self.session.messages.clone(),
                model: self.session.model.clone(),
            };

            let mut stream = provider.stream_chat(request).await?;
            spinner.finish_and_clear();

            println!("{}:", "Forge".cyan().bold());
            
            let mut full_response = String::new();
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        // Stream raw for responsiveness, but we will clean it after
                        print!("{}", chunk.text);
                        full_response.push_str(&chunk.text);
                        io::stdout().flush()?;
                    }
                    Err(e) => {
                        error!(error = ?e, "Streaming error.");
                        break;
                    }
                }
            }
            println!("\n"); // Clear the line after streaming

            // 1. Parse and Clean the response
            let parts = ResponseParser::parse(&full_response);
            let mut cleaned_text = String::new();
            let mut pending_tools = Vec::new();

            for part in parts {
                match part {
                    ResponsePart::Text(t) => cleaned_text.push_str(&t),
                    ResponsePart::Shell(cmd) => pending_tools.push(cmd),
                }
            }

            // 2. Final Render (Markdown)
            // We reprint only if there were tags to hide, to avoid double printing.
            if full_response.contains("<shell>") {
                println!("{}", "--- Refined Output (Tags Hidden) ---".dimmed());
                UI::render_markdown(&cleaned_text);
            } else {
                // If no tags, the streamed output is already the final output.
                // Just ensuring it ends nicely.
            }

            // 3. Handle Actions
            for cmd in pending_tools {
                tool_call_count += 1;
                if tool_call_count > MAX_TOOL_CALLS {
                    println!("\n{} Loop protection triggered.", "⚠".yellow());
                    break;
                }

                println!("\n{} Proposed Action: {}", "➜".yellow().bold(), cmd.green());
                if Confirm::new("Execute?").with_default(false).prompt()? {
                    let tool_spinner = UI::create_spinner("Executing...");
                    match self.tools[0].execute(&cmd).await {
                        Ok(output) => {
                            tool_spinner.finish_and_clear();
                            println!("{} Output received ({} bytes)", "✓".green(), output.len());
                            current_prompt = format!("Tool Output:\n{}", output);
                        }
                        Err(e) => {
                            tool_spinner.finish_and_clear();
                            current_prompt = format!("Tool Error: {}", e);
                        }
                    }
                } else {
                    current_prompt = "User cancelled the tool execution.".to_string();
                }
            }

            self.session.add_message(Role::Assistant, full_response);
            self.session_manager.save(&self.session)?;

            if !current_prompt.is_empty() && current_prompt.starts_with("Tool") {
                continue; 
            }
        }
        Ok(())
    }

    async fn handle_slash_command(&mut self, input: &str) -> Result<()> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        let cmd = parts[0];

        match cmd {
            "/provider" => {
                if let Some(p) = parts.get(1) {
                    if self.factory.get(p).is_some() {
                        self.provider_name = p.to_string();
                        println!("{} Switched provider to: {}", "✓".green(), p.bold());
                    } else {
                        println!("{} Provider '{}' not found or not configured.", "✗".red(), p);
                    }
                } else {
                    println!("{} Usage: /provider <name>", "ℹ".blue());
                }
            }
            "/model" => {
                if let Some(m) = parts.get(1) {
                    self.session.model = Some(m.to_string());
                    println!("{} Switched model to: {}", "✓".green(), m.bold());
                } else {
                    println!("{} Usage: /model <name>", "ℹ".blue());
                }
            }
            "/models" => {
                let spinner = UI::create_spinner("Fetching models...");
                if let Some(provider) = self.factory.get(&self.provider_name) {
                    match provider.list_models().await {
                        Ok(models) => {
                            spinner.finish_and_clear();
                            println!("{}", format!("Available models for {}:", self.provider_name).yellow().bold());
                            for m in models.iter().take(20) {
                                println!("  - {}", m);
                            }
                            if models.len() > 20 {
                                println!("  ... and {} more.", models.len() - 20);
                            }
                        }
                        Err(e) => {
                            spinner.finish_and_clear();
                            println!("{} Failed to fetch models: {}", "✗".red(), e);
                        }
                    }
                }
            }
            "/history" => {
                println!("{} Session Statistics:", "ℹ".blue().bold());
                println!("  - Provider: {}", self.provider_name);
                println!("  - Model:    {}", self.session.model.as_deref().unwrap_or("default"));
                println!("  - Messages: {}", self.session.messages.len());
            }
            "/help" => {
                self.print_help();
            }
            _ => {
                println!("{} Unknown command: {}", "✗".red(), cmd);
                self.print_help();
            }
        }
        Ok(())
    }

    fn print_help(&self) {
        println!("\n{}", "Available Slash Commands:".yellow().bold());
        println!("  {} <name>  - Switch active provider (e.g., google, local, openrouter)", "/provider".cyan());
        println!("  {} <name>     - Switch model for current provider", "/model".cyan());
        println!("  {}           - List available models from the current provider", "/models".cyan());
        println!("  {}         - Show current session statistics", "/history".cyan());
        println!("  {}            - Show this help message", "/help".cyan());
        println!("  {} / {}      - Exit the chat session\n", "exit".cyan(), "quit".cyan());
    }
}
