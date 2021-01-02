use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};

const CONFIG_FILENAME: &str = "config.toml";

pub fn load_settings(folder_path: &Path) -> Result<Settings> {
    let file_path = folder_path.join(CONFIG_FILENAME);
    log::info!("Reading settings from path {}...", file_path.display());
    let settings = if file_path.is_file() {
        let mut settings_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&file_path)
            .context(format!(
                "Failed to open settings file from path {}...",
                file_path.display()
            ))?;
        let mut buf = String::new();
        settings_file.read_to_string(&mut buf).context(format!(
            "Failed to read settings file from path {}...",
            file_path.display()
        ))?;
        toml::de::from_str(&buf).context(format!(
            "Failed to parse settings file from path {}...",
            file_path.display()
        ))?
    } else {
        std::fs::create_dir_all(folder_path)?;
        let settings = Settings::default();
        write_settings(file_path, &settings)?;
        settings
    };

    // TODO: write settings

    Ok(settings)
}

fn write_settings(path: impl AsRef<Path>, settings: &Settings) -> Result<()> {
    log::info!("Writing settings...");
    let path = path.as_ref();
    let mut settings_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)
        .context(format!("Failed to open settings file {}", path.display()))?;
    let string = toml::ser::to_string(settings).context("Failed to serialize settings")?;
    settings_file
        .write(string.as_bytes())
        .context(format!("Failed to write settings file {}", path.display()))?;

    Ok(())
}

/// Settings of the game
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Settings {
    pub window_size: [u16; 2],
    pub invert_mouse: bool,
    pub render_distance: (u64, u64, u64, u64, u64, u64),
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            window_size: [1600, 900],
            invert_mouse: false,
            render_distance: (16, 16, 16, 16, 16, 16),
        }
    }
}
