use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use image::imageops::FilterType;
use mpd_client::{Client, commands, responses::Song};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use ratatui_image::{Resize, StatefulImage, picker::Picker, protocol::StatefulProtocol};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};
use tokio::net::TcpStream;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub mpd: MpdConfig,
    pub paths: PathsConfig,
    pub colors: ColorsConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MpdConfig {
    pub address: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PathsConfig {
    pub music_dir: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ColorsConfig {
    pub border: String,
    pub title: String,
    pub album: String,
    pub artist: String,
    pub status: String,
}

impl Config {
    pub fn load() -> color_eyre::Result<Self> {
        let home = std::env::var("HOME")?;
        let config_dir = PathBuf::from(home).join(".config").join("zarumet");
        let config_path = config_dir.join("config.toml");
        // Check if config file exists
        if !config_path.exists() {
            // Create config directory if it doesn't exist
            std::fs::create_dir_all(&config_dir)?;

            // Create default config
            let default_config = Config::default();

            // Serialize to TOML and write to file
            let toml_string = toml::to_string_pretty(&default_config)?;
            std::fs::write(&config_path, &toml_string)?;

            eprintln!("Created default config file at: {}", config_path.display());

            return Ok(default_config);
        }
        let contents = std::fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
}

impl ColorsConfig {
    /// Parse a hex color string like "#FF5500" into RGB values
    pub fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some((r, g, b))
    }

    pub fn album_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.album)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn status_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.status)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn border_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.border)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn artist_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.artist)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Cyan)
    }

    pub fn title_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.title)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Yellow)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mpd: MpdConfig::default(),
            paths: PathsConfig::default(),
            colors: ColorsConfig::default(),
        }
    }
}

impl Default for MpdConfig {
    fn default() -> Self {
        Self {
            address: "localhost:6600".to_string(),
        }
    }
}

impl Default for PathsConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
        Self {
            music_dir: PathBuf::from(home).join("Music"),
        }
    }
}

impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            border: "#FAE280".to_string(),
            title: "#FAE280".to_string(),
            album: "#FAE280".to_string(),
            artist: "#FAE280".to_string(),
            status: "#FAE280".to_string(),
        }
    }
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new()?.run(terminal).await;
    ratatui::restore();
    result
}

/// The main application which holds the state and logic of the application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    running: bool,
    /// Current song information
    current_song: Option<SongInfo>,
    /// Configuration loaded from TOML file
    config: Config,
}

#[derive(Debug, Clone)]
pub struct SongInfo {
    title: String,
    artist: String,
    album: String,
    album_dir: PathBuf,
}

impl SongInfo {
    fn from_song(song: &Song) -> Self {
        let title = song
            .title()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown Title".to_string());
        let artist = song
            .artists()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown Artist".to_string());
        let album = song
            .album()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown Album".to_string());

        let album_dir = song
            .file_path()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();

        Self {
            title,
            artist,
            album,
            album_dir,
        }
    }
    /// Find cover art using the provided music directory
    pub fn find_cover_art(&self, music_dir: &PathBuf) -> Option<PathBuf> {
        let full_album_path = music_dir.join(&self.album_dir);

        let cover_names = ["cover.jpg", "cover.png", "Cover.jpg", "Cover.png"];

        for name in cover_names {
            let cover_path = full_album_path.join(name);
            if cover_path.exists() {
                return Some(cover_path);
            }
        }

        if let Some(parent_path) = full_album_path.parent() {
            for name in cover_names {
                let cover_path = parent_path.join(name);
                if cover_path.exists() {
                    return Some(cover_path);
                }
            }
        }

        None
    }
}

struct Protocol {
    image: StatefulProtocol,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> color_eyre::Result<Self> {
        let config = Config::load()?;
        Ok(Self {
            running: false,
            current_song: None,
            config,
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;

        // Connect to MPD
        let connection = TcpStream::connect(&self.config.mpd.address).await?;
        let (client, _state_changes) = Client::connect(connection).await?;

        // Set up the image picker and protocol
        let mut picker = Picker::from_query_stdio().unwrap();
        picker.set_background_color([0, 0, 0, 0]);

        // Fetch initial song info
        self.update_current_song(&client).await?;

        let mut current_image_path = self
            .current_song
            .as_ref()
            .and_then(|song| song.find_cover_art(&self.config.paths.music_dir))
            .unwrap_or_default();

        // Create protocol with initial image
        let dyn_img = image::ImageReader::open(&current_image_path)?.decode()?;
        let image = picker.new_resize_protocol(dyn_img);
        let mut protocol = Protocol { image };

        while self.running {
            terminal.draw(|frame| self.render(frame, &mut protocol))?;
            protocol.image.last_encoding_result();

            // Poll for events with a timeout to allow periodic updates
            if event::poll(Duration::from_millis(100))? {
                self.handle_crossterm_events()?;
            }

            // Update song info periodically
            self.update_current_song(&client).await?;

            let new_image_path = self
                .current_song
                .as_ref()
                .and_then(|song| song.find_cover_art(&self.config.paths.music_dir))
                .unwrap_or_default();

            if new_image_path != current_image_path {
                if let Ok(reader) = image::ImageReader::open(&new_image_path) {
                    if let Ok(dyn_img) = reader.decode() {
                        protocol.image = picker.new_resize_protocol(dyn_img);
                        current_image_path = new_image_path;
                    }
                }
            }
        }
        Ok(())
    }

    /// Update the current song information from MPD
    async fn update_current_song(&mut self, client: &Client) -> color_eyre::Result<()> {
        match client.command(commands::CurrentSong).await {
            Ok(Some(song_in_queue)) => {
                self.current_song = Some(SongInfo::from_song(&song_in_queue.song));
            }
            Ok(None) => {
                self.current_song = None;
            }
            Err(_) => {
                // Keep the previous song info on error
            }
        }
        Ok(())
    }

    /// Renders the user interface.
    fn render(&mut self, frame: &mut Frame<'_>, protocol: &mut Protocol) {
        let area = frame.area();

        // Split the area: image on top, song info at bottom
        let chunks = Layout::vertical([
            Constraint::Min(10),   // Image takes most space
            Constraint::Length(5), // Song info takes 5 lines
        ])
        .split(area);

        // Render the album art image (centered)
        let image = StatefulImage::default().resize(Resize::Fit(Some(FilterType::Lanczos3)));

        let image_area = center_area(
            chunks[0],
            Constraint::Percentage(100),
            Constraint::Percentage(100),
        );
        frame.render_stateful_widget(image, image_area, &mut protocol.image);

        // Render the song information
        let song_widget = self.create_song_widget(chunks[1]);
        frame.render_widget(song_widget, chunks[1]);
    }

    /// Create the song information widget
    fn create_song_widget(&self, _area: Rect) -> Paragraph<'_> {
        // Get colors from config
        let album_color = self.config.colors.album_color();
        let artist_color = self.config.colors.artist_color();
        let title_color = self.config.colors.title_color();
        let status_color = self.config.colors.status_color();
        let border_color = self.config.colors.border_color();

        let lines = match &self.current_song {
            Some(song) => vec![
                Line::from(vec![Span::styled(
                    &song.title,
                    Style::default().fg(title_color),
                )]),
                Line::from(vec![Span::styled(
                    &song.artist,
                    Style::default().fg(artist_color),
                )]),
                Line::from(vec![Span::styled(
                    &song.album,
                    Style::default().fg(album_color),
                )]),
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
                        Style::default().fg(status_color),
                    ))
                    .border_style(Style::default().fg(border_color)),
            )
            .centered()
    }

    /// Reads the crossterm events and updates the state of [`App`].
    fn handle_crossterm_events(&mut self) -> color_eyre::Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}

/// Helper function to center a rect within another rect
fn center_area(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}
