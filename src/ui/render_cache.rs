//! Render caching for expensive string operations
//!
//! This module provides caches for strings that are expensive to compute
//! but rarely change (e.g., formatted durations, progress bars, fillers).

use std::collections::HashMap;

/// Maximum width for pre-generated filler strings
const MAX_FILLER_WIDTH: usize = 256;

/// Maximum number of progress bar widths to cache
const MAX_PROGRESS_BAR_WIDTH: usize = 256;

/// Cache for pre-generated filler strings (spaces, dashes, etc.)
#[derive(Debug)]
pub struct FillerCache {
    /// Pre-generated space strings: spaces[n] = " ".repeat(n)
    spaces: Vec<String>,
    /// Pre-generated dash strings: dashes[n] = "─".repeat(n)
    dashes: Vec<String>,
    /// Pre-generated progress bar characters: bars[n] = "━".repeat(n)
    progress_chars: Vec<String>,
}

impl Default for FillerCache {
    fn default() -> Self {
        Self::new()
    }
}

impl FillerCache {
    /// Create a new filler cache with pre-generated strings
    pub fn new() -> Self {
        Self {
            spaces: (0..=MAX_FILLER_WIDTH).map(|n| " ".repeat(n)).collect(),
            dashes: (0..=MAX_FILLER_WIDTH).map(|n| "─".repeat(n)).collect(),
            progress_chars: (0..=MAX_PROGRESS_BAR_WIDTH)
                .map(|n| "━".repeat(n))
                .collect(),
        }
    }

    /// Get a string of `n` spaces
    #[inline]
    pub fn spaces(&self, n: usize) -> &str {
        if n < self.spaces.len() {
            &self.spaces[n]
        } else {
            // Fallback for very wide terminals (shouldn't happen often)
            ""
        }
    }

    /// Get a string of `n` horizontal dashes (─)
    #[inline]
    pub fn dashes(&self, n: usize) -> &str {
        if n < self.dashes.len() {
            &self.dashes[n]
        } else {
            ""
        }
    }

    /// Get a string of `n` progress bar characters (━)
    #[inline]
    pub fn progress_chars(&self, n: usize) -> &str {
        if n < self.progress_chars.len() {
            &self.progress_chars[n]
        } else {
            ""
        }
    }
}

/// Cache for formatted duration strings
#[derive(Debug, Default)]
pub struct DurationCache {
    /// Short format: seconds -> "M:SS" or "MM:SS"
    short: HashMap<u64, String>,
    /// Long format: seconds -> "H:MM:SS"
    long: HashMap<u64, String>,
    /// With prefix spaces: seconds -> "  M:SS"
    prefixed: HashMap<u64, String>,
}

impl DurationCache {
    /// Create a new empty duration cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Get short format duration (M:SS or MM:SS)
    /// Used for song durations in queue, track lists, etc.
    pub fn format_short(&mut self, secs: u64) -> &str {
        self.short.entry(secs).or_insert_with(|| {
            let m = secs / 60;
            let s = secs % 60;
            format!("{}:{:02}", m, s)
        })
    }

    /// Get long format duration (H:MM:SS or M:SS)
    /// Used for album total durations
    pub fn format_long(&mut self, secs: u64) -> &str {
        self.long.entry(secs).or_insert_with(|| {
            let h = secs / 3600;
            let m = (secs % 3600) / 60;
            let s = secs % 60;
            if h > 0 {
                format!("{}:{}:{:02}", h, m, s)
            } else {
                format!("{}:{:02}", m, s)
            }
        })
    }

    /// Get duration with prefix spaces (  M:SS)
    /// Used for track durations with alignment
    #[allow(dead_code)]
    pub fn format_prefixed(&mut self, secs: u64) -> &str {
        self.prefixed.entry(secs).or_insert_with(|| {
            let m = secs / 60;
            let s = secs % 60;
            format!("  {}:{:02}", m, s)
        })
    }

    /// Get the number of cached entries
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.short.len() + self.long.len() + self.prefixed.len()
    }

    /// Check if cache is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.short.is_empty() && self.long.is_empty() && self.prefixed.is_empty()
    }
}

/// Cache for volume bar display strings
#[derive(Debug, Default)]
pub struct VolumeBarCache {
    /// Cached filled bar strings: volume -> "████████"
    filled: HashMap<u8, String>,
    /// Cached empty bar strings: volume -> "██"
    empty: HashMap<u8, String>,
    /// Cached percentage strings: volume -> " 80%"
    percent: HashMap<u8, String>,
}

impl VolumeBarCache {
    /// Create a new volume bar cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the filled portion of the volume bar
    pub fn filled(&mut self, volume: u8) -> &str {
        let bars = (volume / 10) as usize;
        self.filled
            .entry(volume)
            .or_insert_with(|| "█".repeat(bars))
    }

    /// Get the empty portion of the volume bar
    pub fn empty(&mut self, volume: u8) -> &str {
        let bars = (volume / 10) as usize;
        let empty = 10_usize.saturating_sub(bars);
        self.empty
            .entry(volume)
            .or_insert_with(|| "█".repeat(empty))
    }

    /// Get the percentage string
    pub fn percent(&mut self, volume: u8) -> &str {
        self.percent
            .entry(volume)
            .or_insert_with(|| format!(" {}%", volume))
    }
}

/// Master render cache containing all sub-caches
#[derive(Debug)]
pub struct RenderCache {
    /// Pre-generated filler strings
    pub fillers: FillerCache,
    /// Cached duration format strings
    pub durations: DurationCache,
    /// Cached volume bar strings
    pub volume_bars: VolumeBarCache,
}

impl Default for RenderCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderCache {
    /// Create a new render cache with pre-generated fillers
    pub fn new() -> Self {
        Self {
            fillers: FillerCache::new(),
            durations: DurationCache::new(),
            volume_bars: VolumeBarCache::new(),
        }
    }

    /// Log cache statistics for debugging
    #[allow(dead_code)]
    pub fn log_stats(&self) {
        log::debug!(
            "RenderCache stats: durations={}, filler_widths={}",
            self.durations.len(),
            MAX_FILLER_WIDTH
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filler_cache() {
        let cache = FillerCache::new();

        assert_eq!(cache.spaces(0), "");
        assert_eq!(cache.spaces(1), " ");
        assert_eq!(cache.spaces(5), "     ");

        assert_eq!(cache.dashes(0), "");
        assert_eq!(cache.dashes(3), "───");

        assert_eq!(cache.progress_chars(2), "━━");
    }

    #[test]
    fn test_duration_cache() {
        let mut cache = DurationCache::new();

        assert_eq!(cache.format_short(0), "0:00");
        assert_eq!(cache.format_short(65), "1:05");
        assert_eq!(cache.format_short(3661), "61:01");

        assert_eq!(cache.format_long(65), "1:05");
        assert_eq!(cache.format_long(3661), "1:1:01");
        assert_eq!(cache.format_long(7322), "2:2:02");

        assert_eq!(cache.format_prefixed(65), "  1:05");
    }

    #[test]
    fn test_volume_bar_cache() {
        let mut cache = VolumeBarCache::new();

        assert_eq!(cache.filled(50), "█████");
        assert_eq!(cache.empty(50), "█████");
        assert_eq!(cache.percent(50), " 50%");

        assert_eq!(cache.filled(100), "██████████");
        assert_eq!(cache.empty(100), "");
        assert_eq!(cache.percent(100), " 100%");
    }

    #[test]
    fn test_cache_reuse() {
        let mut cache = DurationCache::new();

        // First access computes
        let _ = cache.format_short(120);
        assert_eq!(cache.short.len(), 1);

        // Second access should reuse
        let _ = cache.format_short(120);
        assert_eq!(cache.short.len(), 1);

        // Different value adds to cache
        let _ = cache.format_short(180);
        assert_eq!(cache.short.len(), 2);
    }
}
