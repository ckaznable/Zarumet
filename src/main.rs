// Module declarations
mod app;
mod binds;
mod config;
mod mpd_handler;
mod song;
mod terminal;
mod ui;

use app::cli::Args;
use app::{App, constructor::AppConstructor, main_loop::AppMainLoop};
use clap::Parser;
use terminal::{init_terminal, restore_terminal};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Parse command line arguments
    let args = Args::parse();

    // Initialize terminal
    let terminal = init_terminal()?;

    // Run the application
    let result = App::new(args)?.run(terminal).await;

    // Restore terminal
    restore_terminal()?;
    result
}
