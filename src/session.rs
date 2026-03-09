use crate::providers::{Message, Role};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use anyhow::Result;
use tracing::debug;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Session {
    pub id: String,
    pub provider: String,
    pub model: Option<String>,
    pub messages: Vec<Message>,
}

impl Session {
    pub fn new(id: String, provider: String, model: Option<String>) -> Self {
        Self {
            id,
            provider,
            model,
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, role: Role, content: String) {
        self.messages.push(Message { role, content });
    }

    /// Prune history if it gets too long (Basic implementation)
    pub fn limit_context(&mut self, max_messages: usize) {
        if self.messages.len() > max_messages {
            // Keep the system prompt if it exists at index 0
            let start_idx = if self.messages.first().map(|m| m.role == Role::System).unwrap_or(false) {
                1
            } else {
                0
            };
            
            let to_remove = self.messages.len() - max_messages;
            if to_remove > 0 {
                self.messages.drain(start_idx..(start_idx + to_remove));
            }
        }
    }
}

pub struct SessionManager {
    storage_dir: PathBuf,
}

impl SessionManager {
    pub fn new(storage_dir: PathBuf) -> Result<Self> {
        if !storage_dir.exists() {
            fs::create_dir_all(&storage_dir)?;
        }
        Ok(Self { storage_dir })
    }

    pub fn save(&self, session: &Session) -> Result<()> {
        let path = self.storage_dir.join(format!("{}.json", session.id));
        let json = serde_json::to_string_pretty(session)?;
        fs::write(path, json)?;
        debug!(session_id = %session.id, "Session persisted to disk.");
        Ok(())
    }

    pub fn load(&self, id: &str) -> Result<Session> {
        let path = self.storage_dir.join(format!("{}.json", id));
        let json = fs::read_to_string(path)?;
        let session = serde_json::from_str(&json)?;
        Ok(session)
    }

    pub fn list_sessions(&self) -> Result<Vec<String>> {
        let mut sessions = Vec::new();
        for entry in fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            if let Some(ext) = entry.path().extension() {
                if ext == "json" {
                    if let Some(name) = entry.path().file_stem() {
                        sessions.push(name.to_string_lossy().into_owned());
                    }
                }
            }
        }
        Ok(sessions)
    }
}
