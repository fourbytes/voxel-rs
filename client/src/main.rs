use anyhow::Result;
use std::path::Path;

mod fps;
mod gui;
mod input;
mod render;
mod settings;
mod singleplayer;
mod texture;
mod ui;
mod window;
mod world;

fn main() -> Result<()> {
    env_logger::init();

    log::info!("Starting up...");
    let config_folder = Path::new("config");
    let config_file = Path::new("config/settings.toml");
    let settings = settings::load_settings(&config_folder, &config_file)?;
    log::info!("Current settings: {:?}", settings);

    window::open_window(
        settings,
        // Box::new(singleplayer::SinglePlayer::new_factory(Box::new(client))),
        ui::mainmenu::MainMenu::new_factory(),
    )
}
