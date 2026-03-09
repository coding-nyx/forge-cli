use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use tracing::{info, warn};

pub struct ContextManager;

impl ContextManager {
    /// Read the contents of specific files and format them as context
    pub fn read_files(paths: &[String]) -> anyhow::Result<String> {
        let mut context = String::new();
        for path_str in paths {
            let path = Path::new(path_str);
            if !path.exists() {
                warn!(path = %path_str, "File not found. Skipping context.");
                continue;
            }

            if path.is_dir() {
                warn!(path = %path_str, "Path is a directory. Skipping context.");
                continue;
            }

            // Simple binary check (read first 512 bytes for nulls)
            if Self::is_binary(path)? {
                warn!(path = %path_str, "File appears to be binary. Skipping context.");
                continue;
            }

            let content = fs::read_to_string(path)?;
            context.push_str(&format!("\n--- FILE: {} ---\n{}\n", path_str, content));
            info!(path = %path_str, "Added file to context.");
        }
        Ok(context)
    }

    /// Generate a summary of the current repository structure
    pub fn get_repo_map(root: &Path) -> anyhow::Result<String> {
        let mut map = String::new();
        map.push_str("--- PROJECT REPOSITORY MAP ---\n");

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !Self::is_ignored(e.path()))
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                let rel_path = path.strip_prefix(root).unwrap_or(path);
                map.push_str(&format!("  - {}\n", rel_path.display()));
            }
        }
        Ok(map)
    }

    fn is_ignored(path: &Path) -> bool {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        name == ".git" || name == "target" || name == "node_modules" || name == ".gemini"
    }

    fn is_binary(path: &Path) -> anyhow::Result<bool> {
        let mut file = fs::File::open(path)?;
        let mut buffer = [0u8; 512];
        use std::io::Read;
        let n = file.read(&mut buffer)?;
        Ok(buffer[..n].iter().any(|&b| b == 0))
    }
}
