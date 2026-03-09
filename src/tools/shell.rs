use crate::tools::Tool;
use async_trait::async_trait;
use std::process::Command;
use anyhow::Result;
use tracing::info;

pub struct ShellTool;

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command (e.g., ls, cargo test, git status)"
    }

    async fn execute(&self, args: &str) -> Result<String> {
        info!(command = %args, "Executing shell command...");
        
        let output = if cfg!(target_os = "windows") {
            Command::new("powershell")
                .arg("-Command")
                .arg(args)
                .output()?
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(args)
                .output()?
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(stdout)
        } else {
            Ok(format!("Error: {}\n{}", stderr, stdout))
        }
    }
}
