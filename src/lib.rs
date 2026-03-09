pub mod cli;
pub mod providers;
pub mod session;
pub mod context;
pub mod tools;
pub mod ui;
pub mod orchestrator;
pub mod config;

// Exposing core types for easier access
pub use providers::base::{ChatRequest, ChatChunk, ForgeProvider};
pub use session::{Session, SessionManager};
pub use context::ContextManager;
pub use tools::{Tool, ShellTool};
pub use ui::UI;
pub use orchestrator::Orchestrator;
pub use config::{Config, ApiConfig, LocalConfig};
