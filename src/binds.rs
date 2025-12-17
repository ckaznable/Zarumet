use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mpd_client::{client::CommandError, commands, responses::PlayState};

/// Key binding definitions for MPD controls
pub struct KeyBinds;

impl KeyBinds {
    /// Handle key events and return corresponding MPD commands
    pub fn handle_key(key: KeyEvent) -> Option<MPDAction> {
        match (key.modifiers, key.code) {
            // Playback controls
            (_, KeyCode::Char(' ')) => Some(MPDAction::TogglePlayPause),
            (_, KeyCode::Char('p')) => Some(MPDAction::TogglePlayPause),
            (_, KeyCode::Char('s')) => Some(MPDAction::Stop),
            (_, KeyCode::Char('>')) | (_, KeyCode::Char('n')) => Some(MPDAction::Next),
            (_, KeyCode::Char('<')) | (_, KeyCode::Char('b')) => Some(MPDAction::Previous),

            // Volume controls
            (_, KeyCode::Char('=')) | (_, KeyCode::Char('+')) => Some(MPDAction::VolumeUp),
            (_, KeyCode::Char('-')) | (_, KeyCode::Char('_')) => Some(MPDAction::VolumeDown),
            (_, KeyCode::Char('m')) => Some(MPDAction::ToggleMute),

            // Seek controls
            (_, KeyCode::Char('l')) => Some(MPDAction::SeekForward),
            (_, KeyCode::Char('h')) => Some(MPDAction::SeekBackward),
            (_, KeyCode::Right) => Some(MPDAction::SeekForward),
            (_, KeyCode::Left) => Some(MPDAction::SeekBackward),

            // Queue controls
            (_, KeyCode::Char('c')) => Some(MPDAction::ClearQueue),
            (_, KeyCode::Char('r')) => Some(MPDAction::Random),
            (_, KeyCode::Char('z')) => Some(MPDAction::Repeat),
            (_, KeyCode::Char('x')) => Some(MPDAction::Single),

            // Application controls
            (_, KeyCode::Esc) | (_, KeyCode::Char('q')) => Some(MPDAction::Quit),
            (KeyModifiers::CONTROL, KeyCode::Char('C')) => Some(MPDAction::Quit),
            (_, KeyCode::Char('R')) => Some(MPDAction::Refresh),

            _ => None,
        }
    }
}

/// Actions that can be performed on MPD
#[derive(Debug, Clone)]
pub enum MPDAction {
    // Playback
    TogglePlayPause,
    Stop,
    Next,
    Previous,

    // Volume
    VolumeUp,
    VolumeDown,
    ToggleMute,

    // Seeking
    SeekForward,
    SeekBackward,

    // Queue options
    ClearQueue,
    Random,
    Repeat,
    Single,

    // Application
    Quit,
    Refresh,
}

impl MPDAction {
    /// Execute the action on the MPD client
    pub async fn execute(&self, client: &mpd_client::Client) -> Result<(), CommandError> {
        match self {
            MPDAction::TogglePlayPause => {
                let status = client.command(commands::Status).await?;
                match status.state {
                    PlayState::Playing => {
                        client.command(commands::SetPause(true)).await?;
                    }
                    _ => {
                        client.command(commands::Play::current()).await?;
                    }
                }
            }
            MPDAction::Stop => {
                client.command(commands::Stop).await?;
            }
            MPDAction::Next => {
                client.command(commands::Next).await?;
            }
            MPDAction::Previous => {
                client.command(commands::Previous).await?;
            }
            MPDAction::VolumeUp => {
                let status = client.command(commands::Status).await?;
                let new_volume = std::cmp::min(100, status.volume + 5);
                client.command(commands::SetVolume(new_volume)).await?;
            }
            MPDAction::VolumeDown => {
                let status = client.command(commands::Status).await?;
                let new_volume = std::cmp::max(0, status.volume - 5);
                client.command(commands::SetVolume(new_volume)).await?;
            }
            MPDAction::ToggleMute => {
                let status = client.command(commands::Status).await?;
                if status.volume > 0 {
                    client.command(commands::SetVolume(0)).await?;
                } else {
                    client.command(commands::SetVolume(50)).await?;
                }
            }
            MPDAction::SeekForward => {
                client
                    .command(commands::Seek(commands::SeekMode::Forward(
                        std::time::Duration::from_secs(5),
                    )))
                    .await?;
            }
            MPDAction::SeekBackward => {
                client
                    .command(commands::Seek(commands::SeekMode::Backward(
                        std::time::Duration::from_secs(5),
                    )))
                    .await?;
            }
            MPDAction::ClearQueue => {
                client.command(commands::ClearQueue).await?;
            }
            MPDAction::Random => {
                let status = client.command(commands::Status).await?;
                client.command(commands::SetRandom(!status.random)).await?;
            }
            MPDAction::Repeat => {
                let status = client.command(commands::Status).await?;
                client.command(commands::SetRepeat(!status.repeat)).await?;
            }
            MPDAction::Single => {
                let status = client.command(commands::Status).await?;
                // Toggle single mode
                let new_single = match status.single {
                    commands::SingleMode::Enabled => commands::SingleMode::Disabled,
                    _ => commands::SingleMode::Enabled,
                };
                client.command(commands::SetSingle(new_single)).await?;
            }
            MPDAction::Quit | MPDAction::Refresh => {
                // These are handled by the main application
            }
        }
        Ok(())
    }
}
