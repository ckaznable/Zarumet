use ratatui::{
    style::Style,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::config::Config;

pub fn create_top_box<'a>(config: &Config) -> Paragraph<'a> {
    let border_color = config.colors.border_color();
    let text_color = config.colors.song_title_color();

    let content = "Placeholder".to_string();

    Paragraph::new(content)
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .style(Style::default().fg(text_color))
        .centered()
}
