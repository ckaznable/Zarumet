mod binds;
mod cli;
mod config;
mod song;
mod ui;

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::executor::block_on;
use mpd_client::{Client, commands};
use ratatui::DefaultTerminal;
use ratatui_image::picker::Picker;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpStream;

use cli::Args;
use config::Config;
use song::SongInfo;
use ui::Protocol;
use binds::{KeyBinds, MPDAction};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Parse command line arguments
    let args = Args::parse();

    // Initialize terminal with explicit crossterm configuration for full control
    execute!(
        std::io::stdout(),
        EnterAlternateScreen
    )?;
    enable_raw_mode()?;
    
    let terminal = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;
    let result = App::new(args)?.run(terminal).await;
    
    // Restore terminal
    execute!(
        std::io::stdout(),
        LeaveAlternateScreen
    )?;
    disable_raw_mode()?;
    result
}

/// The main application which holds the state and logic of the application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    running: bool,
    /// Current song information
    current_song: Option<SongInfo>,
    /// MPD queue information
    queue: Vec<SongInfo>,
    /// Currently selected queue item index
    selected_queue_index: Option<usize>,
    /// Configuration loaded from TOML file
    config: Config,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new(args: Args) -> color_eyre::Result<Self> {
        let mut config = Config::load(args.config)?;

        if let Some(address) = args.address {
            config.mpd.address = address;
        }

        Ok(Self {
            running: false,
            current_song: None,
            queue: Vec::new(),
            selected_queue_index: Some(0), // Select top song by default
            config,
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;

        // Connect to MPD
        let connection = TcpStream::connect(&self.config.mpd.address).await?;
        let (client, _state_changes) = Client::connect(connection).await?;

        match SongInfo::set_max_art_size(&client, 5 * 1024 * 1024).await {
            Ok(_) => {}
            Err(e) => eprintln!("Failed to set MPD binary limit: {}", e),
        }

        // Set up the image picker and protocol
        let mut picker = Picker::from_query_stdio().unwrap();
        picker.set_background_color([0, 0, 0, 0]);

        // Fetch initial song info
        self.update_current_song(&client).await?;

        // Track the current song's file path (not the image path)
        let mut current_song_file: Option<PathBuf> = self
            .current_song
            .as_ref()
            .map(|song| song.file_path.clone());

        // Try to get initial image
        let initial_image = self
            .current_song
            .as_ref()
            .and_then(|song| block_on(song.load_cover(&client)));

        // Create protocol with initial image (if available)
        let mut protocol = Protocol {
            image: initial_image
                .as_ref()
                .and_then(|raw_data| {
                    image::ImageReader::new(Cursor::new(raw_data))
                        .with_guessed_format()
                        .ok()
                })
                .and_then(|reader| reader.decode().ok())
                .map(|dyn_img| picker.new_resize_protocol(dyn_img)),
        };

        while self.running {
            terminal.draw(|frame| {
                ui::render(
                    frame,
                    &mut protocol,
                    &self.current_song,
                    &self.queue,
                    self.selected_queue_index,
                    &self.config,
                )
            })?;

            if let Some(ref mut img) = protocol.image {
                img.last_encoding_result();
            }

            // Poll for events with a timeout to allow periodic updates
            if event::poll(Duration::from_millis(100))? {
                self.handle_crossterm_events(&client).await?;
            }

            // Update song info, queue, and status periodically
            self.run_updates(&client).await?;

            // Check if the song changed (not just the image path)
            let new_song_file: Option<PathBuf> = self
                .current_song
                .as_ref()
                .map(|song| song.file_path.clone());

            if new_song_file != current_song_file {
                // Song changed, reload the cover art
                let new_image = self
                    .current_song
                    .as_ref()
                    .and_then(|song| block_on(song.load_cover(&client)));

                protocol.image = new_image
                    .as_ref()
                    .and_then(|raw_data| {
                        image::ImageReader::new(Cursor::new(raw_data))
                            .with_guessed_format()
                            .ok()
                    })
                    .and_then(|reader| reader.decode().ok())
                    .map(|dyn_img| picker.new_resize_protocol(dyn_img));

                current_song_file = new_song_file;
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

    /// Run update functions concurrently with optimized result processing
    async fn run_updates(&mut self, client: &Client) -> color_eyre::Result<()> {
        // Run MPD commands concurrently
        let (current_song_result, queue_songs, status) = tokio::try_join!(
            client.command(commands::CurrentSong),
            client.command(commands::Queue),
            client.command(commands::Status)
        )?;

        // Process current song result
        match current_song_result {
            Some(song_in_queue) => {
                self.current_song = Some(SongInfo::from_song(&song_in_queue.song));
            }
            None => {
                self.current_song = None;
            }
        }

        // Process queue result
        self.queue = queue_songs
            .into_iter()
            .map(|song_in_queue| SongInfo::from_song(&song_in_queue.song))
            .collect();
        
        // Update selected index to stay within bounds
        if let Some(selected) = self.selected_queue_index {
            if selected >= self.queue.len() {
                self.selected_queue_index = if self.queue.is_empty() {
                    None
                } else {
                    Some(self.queue.len().saturating_sub(1))
                };
            }
        }

        // Process status result
        let progress = match (status.elapsed, status.duration) {
            (Some(elapsed), Some(duration)) => Some(elapsed.as_secs_f64() / duration.as_secs_f64()),
            _ => None,
        };

        if let Some(ref mut song) = self.current_song {
            song.update_playback_info(Some(status.state), progress);
            song.update_time_info(status.elapsed, status.duration);
        }

        Ok(())
    }

    /// Reads the crossterm events and updates the state of [`App`].
    async fn handle_crossterm_events(&mut self, client: &mpd_client::Client) -> color_eyre::Result<()> {
        // Try direct event reading to bypass any terminal interference
        match crossterm::event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.on_key_event(key, client).await?;
            }
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    async fn on_key_event(&mut self, key: KeyEvent, client: &mpd_client::Client) -> color_eyre::Result<()> {
        if let Some(action) = KeyBinds::handle_key(key) {
            match action {
                MPDAction::QueueUp => {
                    if let Some(selected) = self.selected_queue_index {
                        if selected > 0 {
                            self.selected_queue_index = Some(selected - 1);
                        }
                    } else if !self.queue.is_empty() {
                        self.selected_queue_index = Some(0);
                    }
                }
                MPDAction::QueueDown => {
                    if let Some(selected) = self.selected_queue_index {
                        if selected < self.queue.len().saturating_sub(1) {
                            self.selected_queue_index = Some(selected + 1);
                        }
                    } else if !self.queue.is_empty() {
                        self.selected_queue_index = Some(0);
                    }
                }
                MPDAction::PlaySelected => {
                    if let Some(selected) = self.selected_queue_index {
                        if selected < self.queue.len() {
                            // Play the song at the selected position in the queue
                            let song_position: mpd_client::commands::SongPosition = selected.into();
                            if let Err(e) = client.command(mpd_client::commands::Play::song(song_position)).await {
                                eprintln!("Error playing selected song: {}", e);
                            }
                        }
                    }
                }
                MPDAction::MoveUpInQueue => {
                    if let Some(selected) = self.selected_queue_index {
                        if selected > 0 && selected < self.queue.len() {
                            // Move song up in queue (from position `selected` to `selected - 1`)
                            let from_pos: mpd_client::commands::SongPosition = selected.into();
                            let to_pos: mpd_client::commands::SongPosition = (selected - 1).into();
                            if let Err(e) = client.command(mpd_client::commands::Move::position(from_pos).to_position(to_pos)).await {
                                eprintln!("Error moving song up in queue: {}", e);
                            } else {
                                // Update selected index to follow the moved song
                                self.selected_queue_index = Some(selected - 1);
                            }
                        }
                    }
                }
                MPDAction::MoveDownInQueue => {
                    if let Some(selected) = self.selected_queue_index {
                        if selected < self.queue.len().saturating_sub(1) {
                            // Move song down in queue (from position `selected` to `selected + 1`)
                            let from_pos: mpd_client::commands::SongPosition = selected.into();
                            let to_pos: mpd_client::commands::SongPosition = (selected + 1).into();
                            if let Err(e) = client.command(mpd_client::commands::Move::position(from_pos).to_position(to_pos)).await {
                                eprintln!("Error moving song down in queue: {}", e);
                            } else {
                                // Update selected index to follow the moved song
                                self.selected_queue_index = Some(selected + 1);
                            }
                        }
                    }
                }
                MPDAction::Quit => self.quit(),
                MPDAction::Refresh => {
                    // Force refresh by updating current song and queue
                    // This will be handled in the next update cycle
                }
                _ => {
                    // Execute MPD command for other actions
                    if let Err(e) = action.execute(client).await {
                        eprintln!("Error executing MPD command: {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
