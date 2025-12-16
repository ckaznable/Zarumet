use image::imageops::FilterType;
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use ratatui_image::{Resize, StatefulImage, protocol::StatefulProtocol};

use crate::config::Config;
use crate::song::SongInfo;

pub struct Protocol {
    pub image: Option<StatefulProtocol>,
}

/// Renders the user interface.
pub fn render(
    frame: &mut Frame<'_>,
    protocol: &mut Protocol,
    current_song: &Option<SongInfo>,
    config: &Config,
) {
    let area = frame.area();

    // Split the area horizontally: left box, right content
    let horizontal_chunks = Layout::horizontal([
        Constraint::Percentage(45), // Left box takes 25% of width
        Constraint::Percentage(55), // Right content takes 75% of width
    ])
    .split(area);

    let left_vertical_chunks = Layout::vertical([
        Constraint::Percentage(100), // Top content takes 100% of height left
        Constraint::Length(3),       // Bottom content takes 3 lines
    ])
    .split(horizontal_chunks[0]);

    // Extract play_state and progress from current_song
    let (play_state, progress, elapsed, duration) = if let Some(song) = current_song {
        (song.play_state, song.progress, song.elapsed, song.duration)
    } else {
        (None, None, None, None)
    };

    // Render widgets in left vertical split
    let left_box_top = create_left_box_top(config);
    frame.render_widget(left_box_top, left_vertical_chunks[0]);

    // Render widgets in left vertical split
    let left_box_bottom = create_left_box_bottom(&play_state, progress, elapsed, duration, config);
    frame.render_widget(left_box_bottom, left_vertical_chunks[1]);

    // Split the right area vertically: image on top, song info at bottom
    let right_vertical_chunks = Layout::vertical([
        Constraint::Min(10),   // Image takes most space
        Constraint::Length(4), // Song info takes 4 lines
    ])
    .split(horizontal_chunks[1]);

    let image_area = center_area(
        right_vertical_chunks[0],
        Constraint::Percentage(100),
        Constraint::Percentage(100),
    );

    // Only render image if we have one
    if let Some(ref mut img) = protocol.image {
        let image = StatefulImage::default().resize(Resize::Scale(Some(FilterType::Lanczos3)));
        frame.render_stateful_widget(image, image_area, img);
    } else {
        let centered_area = center_area(image_area, Constraint::Length(12), Constraint::Length(1));

        let placeholder = Paragraph::new("No album art")
            .centered()
            .style(Style::default().dark_gray());
        frame.render_widget(placeholder, centered_area);
    }

    // Render the song information
    let song_widget = create_song_widget(current_song, config);
    frame.render_widget(song_widget, right_vertical_chunks[1]);
}

/// Format duration as MM:SS
fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

/// Create the left side box widget
fn create_left_box_bottom(
    play_state: &Option<mpd_client::responses::PlayState>,
    progress: Option<f64>,
    elapsed: Option<std::time::Duration>,
    duration: Option<std::time::Duration>,
    config: &Config,
) -> impl ratatui::widgets::Widget {
    let border_color = config.colors.border_color();
    let border_title_color = config.colors.border_title_color();
    let song_title_color = config.colors.song_title_color();

    let state_text = match play_state {
        Some(mpd_client::responses::PlayState::Playing) => "▶",
        Some(mpd_client::responses::PlayState::Paused) => "⏸",
        Some(mpd_client::responses::PlayState::Stopped) => "⏹",
        None => "⏹",
    };

    let progress_value = progress.unwrap_or(0.0);
    let progress_percentage = (progress_value * 100.0) as u16;

    struct DynamicProgressBar {
        state_text: String,
        progress_percentage: u16,
        border_title_color: Style,
        border_color: Style,
        song_title_color: Style,
        elapsed: Option<std::time::Duration>,
        duration: Option<std::time::Duration>,
    }

    impl ratatui::widgets::Widget for DynamicProgressBar {
        fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
            let block = Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .title(Span::styled(" Progress ", self.border_title_color))
                .border_style(self.border_color);

            let inner = block.inner(area);
            block.render(area, buf);

            // Format time display
            let time_text = match (self.elapsed, self.duration) {
                (Some(elapsed), Some(duration)) => {
                    format!(
                        " {}/{} ",
                        format_duration(elapsed),
                        format_duration(duration)
                    )
                }
                (Some(elapsed), None) => format!(" {}/--:-- ", format_duration(elapsed)),
                (None, Some(duration)) => format!(" --:--/{} ", format_duration(duration)),
                (None, None) => " --:--/--:-- ".to_string(),
            };
            let time_width = time_text.len();

            // Calculate available width for progress bar
            let border_title_width = 1; // border_title icon
            let percentage_width = if self.progress_percentage >= 100 {
                3
            } else {
                2
            }; // "XX%" or "X%"
            let spacing_width = 2; // Spaces around percentage
            let total_text_width =
                border_title_width + percentage_width + spacing_width + time_width;

            let bar_width = inner.width.saturating_sub(total_text_width as u16) as usize;
            let filled = (self.progress_percentage as usize * bar_width / 100).min(bar_width);
            let empty = bar_width.saturating_sub(filled);

            let content = Line::from(vec![
                Span::styled(format!(" {} ", self.state_text), self.song_title_color),
                Span::styled(
                    format!("{}%", self.progress_percentage),
                    self.song_title_color,
                ),
                Span::styled(" ", self.song_title_color),
                Span::styled("━".repeat(filled), self.border_color),
                Span::styled("━".repeat(empty), self.border_title_color),
                Span::styled(time_text, self.song_title_color),
            ]);

            let paragraph = Paragraph::new(content).centered();
            paragraph.render(inner, buf);
        }
    }

    DynamicProgressBar {
        state_text: state_text.to_string(),
        progress_percentage,
        border_title_color: Style::default().fg(border_title_color),
        border_color: Style::default().fg(border_color),
        song_title_color: Style::default().fg(song_title_color),
        elapsed,
        duration,
    }
}

fn create_left_box_top<'a>(config: &Config) -> Paragraph<'a> {
    let border_color = config.colors.border_color();
    let border_title_color = config.colors.border_title_color();

    Paragraph::new("Controls\n\n↑/↓ - Volume\n←/→ - Seek\nSpace - Play/Pause\nq - Quit")
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Controls ",
                    Style::default().fg(border_title_color),
                ))
                .border_style(Style::default().fg(border_color)),
        )
        .centered()
}

/// Create the song information widget
fn create_song_widget<'a>(current_song: &'a Option<SongInfo>, config: &Config) -> Paragraph<'a> {
    // Get colors from config
    let album_color = config.colors.album_color();
    let artist_color = config.colors.artist_color();
    let song_title_color = config.colors.song_title_color();
    let border_title_color = config.colors.border_title_color();
    let border_color = config.colors.border_color();

    let lines = match current_song {
        Some(song) => vec![
            Line::from(vec![Span::styled(
                &song.title,
                Style::default().fg(song_title_color),
            )]),
            Line::from(vec![
                Span::styled(&song.artist, Style::default().fg(artist_color)),
                Span::styled(" - ", Style::default().fg(border_title_color)),
                Span::styled(&song.album, Style::default().fg(album_color)),
            ]),
        ],
        None => vec![Line::from("No song playing").dark_gray()],
    };

    Paragraph::new(lines)
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Now Playing ",
                    Style::default().fg(border_title_color),
                ))
                .border_style(Style::default().fg(border_color)),
        )
        .centered()
}

/// Helper function to center a rect within another rect
pub fn center_area(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}
