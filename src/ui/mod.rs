pub mod albums_renderer;
pub mod menu;
pub mod renderer;
pub mod utils;
pub mod widgets;
pub mod width_cache;

pub use renderer::render;
pub use utils::{DisplayItem, Protocol, compute_album_display_list};
pub use width_cache::WidthCache;

use std::cell::RefCell;

thread_local! {
    /// Global width cache for Unicode string width calculations
    /// Using RefCell for interior mutability within single-threaded ratatui context
    pub static WIDTH_CACHE: RefCell<WidthCache> = RefCell::new(WidthCache::new());
}
