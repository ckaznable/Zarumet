use image::ImageReader;
use lofty::file::TaggedFileExt;
use lofty::picture::PictureType;
use lofty::probe::Probe;
use mpd_client::responses::Song;
use std::io::Cursor;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SongInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_dir: PathBuf,
    pub file_path: PathBuf,
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

        let file_path = song.file_path().to_path_buf();

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
            file_path,
        }
    }
    /// Find cover art using the provided music directory
    pub fn find_cover_art(&self, music_dir: &PathBuf) -> Option<PathBuf> {
        let full_album_path = music_dir.join(&self.album_dir);

        let cover_names = [
            "cover.jpg",
            "cover.png",
            "Cover.jpg",
            "Cover.png",
            "folder.jpg",
            "folder.png",
            "Folder.jpg",
            "Folder.png",
            "album.jpg",
            "album.png",
            "Album.jpg",
            "Album.png",
            "front.jpg",
            "front.png",
            "Front.jpg",
            "Front.png",
            "art.jpg",
            "art.png",
        ];

        // First, search the album directory
        for name in cover_names {
            let cover_path = full_album_path.join(name);
            if cover_path.exists() {
                return Some(cover_path);
            }
        }

        // Then, search one directory up
        if let Some(parent_path) = full_album_path.parent() {
            for name in cover_names {
                let cover_path = parent_path.join(name);
                if cover_path.exists() {
                    return Some(cover_path);
                }
            }
        }

        // Finally, try to extract embedded art from the audio file
        let full_file_path = music_dir.join(&self.file_path);
        self.extract_embedded_art(&full_file_path)
    }

    fn extract_embedded_art(&self, audio_path: &PathBuf) -> Option<PathBuf> {
        let tagged_file = Probe::open(audio_path).ok()?.read().ok()?;

        // Search through all tags for pictures
        for tag in tagged_file.tags() {
            let pictures = tag.pictures();

            if let Some(pic) = pictures
                .iter()
                .find(|p| p.pic_type() == PictureType::CoverFront)
                .or_else(|| pictures.first())
            {
                // Decode the image data and convert to PNG
                let img = ImageReader::new(Cursor::new(pic.data()))
                    .with_guessed_format()
                    .ok()?
                    .decode()
                    .ok()?;

                // Save to cache as PNG
                let cache_dir = std::env::var("HOME").ok()?;
                let cache_path = PathBuf::from(cache_dir).join(".cache").join("zarumet");
                std::fs::create_dir_all(&cache_path).ok()?;

                let cover_path = cache_path.join("current_cover.png");
                img.save_with_format(&cover_path, image::ImageFormat::Png)
                    .ok()?;

                return Some(cover_path);
            }
        }

        None
    }
}
