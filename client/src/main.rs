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
    let game_data_path = Path::new("game_data");
    let settings = settings::load_settings(&game_data_path)?;
    log::info!("Current settings: {:?}", settings);

    window::open_window(
        settings,
        // Box::new(singleplayer::SinglePlayer::new_factory(Box::new(client))),
        ui::mainmenu::MainMenu::new_factory(),
    )
}
