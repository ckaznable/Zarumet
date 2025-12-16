use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub mpd: MpdConfig,
    pub colors: ColorsConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MpdConfig {
    pub address: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ColorsConfig {
    pub border: String,
    pub song_title: String,
    pub album: String,
    pub artist: String,
    pub border_title: String,
    pub progress_filled: String,
    pub progress_empty: String,
    pub paused: String,
    pub playing: String,
    pub stopped: String,
    pub time_separator: String,
    pub time_duration: String,
    pub time_elapsed: String,
}

impl Config {
    pub fn load(config_path: Option<PathBuf>) -> color_eyre::Result<Self> {
        let config_path = match config_path {
            Some(path) => path,
            None => {
                let home = std::env::var("HOME")?;
                PathBuf::from(home)
                    .join(".config")
                    .join("zarumet")
                    .join("config.toml")
            }
        };

        // Check if config file exists
        if !config_path.exists() {
            // Create config directory if it doesn't exist
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

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

    pub fn time_elapsed(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.time_elapsed)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn time_duration(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.time_duration)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn time_separator(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.time_separator)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn paused(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.paused)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn playing(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.playing)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn stopped(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.stopped)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn album_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.album)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn progress_filled_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.progress_filled)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Green)
    }

    pub fn progress_empty_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.progress_empty)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Black)
    }

    pub fn border_title_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.border_title)
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

    pub fn song_title_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.song_title)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Yellow)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mpd: MpdConfig::default(),
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

impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            album: "#26a0a1".to_string(),
            artist: "#d67751".to_string(),
            song_title: "#fae280".to_string(),
            border: "#fae280".to_string(),
            border_title: "#8193af".to_string(),
            playing: "#fae280".to_string(),
            paused: "#fae280".to_string(),
            stopped: "#fae280".to_string(),
            progress_filled: "#26a0a1".to_string(),
            progress_empty: "#1b1d0e".to_string(),
            time_elapsed: "#c6bb69".to_string(),
            time_separator: "#c6bb69".to_string(),
            time_duration: "#c6bb69".to_string(),
        }
    }
}
