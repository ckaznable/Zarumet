use mpd_client::{Client, commands};

use super::App;
use crate::song::SongInfo;

/// Trait for MPD-related updates
pub trait MPDUpdates {
    async fn run_updates(&mut self, client: &Client) -> color_eyre::Result<()>;
}

impl MPDUpdates for App {
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

        // Update selected index to stay within bounds and select first item if queue was previously empty
        match self.queue_list_state.selected() {
            Some(selected) => {
                // If we have a selection, keep it within bounds
                if selected >= self.queue.len() {
                    if self.queue.is_empty() {
                        self.queue_list_state.select(None);
                    } else {
                        self.queue_list_state
                            .select(Some(self.queue.len().saturating_sub(1)));
                    }
                }
            }
            None => {
                // If we have no selection and queue is not empty, select first item
                if !self.queue.is_empty() {
                    self.queue_list_state.select(Some(0));
                }
            }
        }
        self.selected_queue_index = self.queue_list_state.selected();

        // Process status result
        let progress = match (status.elapsed, status.duration) {
            (Some(elapsed), Some(duration)) => Some(elapsed.as_secs_f64() / duration.as_secs_f64()),
            _ => None,
        };

        // Store the status for UI components
        self.mpd_status = Some(status.clone());

        if let Some(ref mut song) = self.current_song {
            song.update_playback_info(Some(status.state), progress);
            song.update_time_info(status.elapsed, status.duration);
        }

        Ok(())
    }
}
