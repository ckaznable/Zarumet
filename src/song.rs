use mpd_client::responses::Song;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SongInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_dir: PathBuf,
}

impl SongInfo {
    pub fn from_song(song: &Song) -> Self {
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
