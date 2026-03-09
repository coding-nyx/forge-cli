use termimad::MadSkin;
pub use termimad::crossterm::style::Color;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub struct UI;

impl UI {
    pub fn get_skin() -> MadSkin {
        let mut skin = MadSkin::default();
        skin.bold.set_fg(Color::Cyan);
        skin.italic.set_fg(Color::Magenta);
        skin.inline_code.set_bg(Color::AnsiValue(237));
        skin.code_block.set_bg(Color::AnsiValue(237));
        skin.set_headers_fg(Color::Yellow);
        skin
    }

    pub fn render_markdown(text: &str) {
        let skin = Self::get_skin();
        skin.print_text(text);
    }

    pub fn create_spinner(msg: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_style(
            ProgressStyle::with_template("{spinner:.blue} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_message(msg.to_string());
        pb
    }
}
