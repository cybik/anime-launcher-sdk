use std::path::PathBuf;

use crate::games::common;
use crate::integrations::steam;

pub const FOLDER_NAME: &str = "honkers-launcher";

/// Get assumed Steam Prefix install path
///
/// Generate a sane, possible, "relative to the prefix's C:\ root" install target for games that
///  need such a location to install the game(s) in.
pub fn base_game_install_dir() -> anyhow::Result<PathBuf> {
    match steam::steam_managed_installed_game() {
        Some(val) => Ok(PathBuf::from(val)), // We're handling a pure Steam install. Neat.
        None => common::base_install_dir(launcher_dir().unwrap())
    }
}

/// Get default launcher dir path
/// 
/// If `LAUNCHER_FOLDER` variable is set, then its value will be returned. Otherwise return `$HOME/.local/share/honkers-launcher`
pub fn launcher_dir() -> anyhow::Result<PathBuf> {
    if let Ok(folder) = std::env::var("LAUNCHER_FOLDER") {
        return Ok(folder.into());
    }

    Ok(std::env::var("XDG_DATA_HOME")
        .or_else(|_| std::env::var("HOME")
        .map(|home| home + "/.local/share"))
        .map(|home| PathBuf::from(home)
        .join(FOLDER_NAME))?)
}

/// Get launcher's cache dir path
/// 
/// If `CACHE_FOLDER` variable is set, then its value will be returned. Otherwise return `$HOME/.cache/honkers-launcher`
pub fn cache_dir() -> anyhow::Result<PathBuf> {
    if let Ok(folder) = std::env::var("CACHE_FOLDER") {
        return Ok(folder.into());
    }

    Ok(std::env::var("XDG_CACHE_HOME")
        .or_else(|_| std::env::var("HOME")
        .map(|home| home + "/.cache"))
        .map(|home| PathBuf::from(home)
        .join(FOLDER_NAME))?)
}

/// Get config file path
/// 
/// Default is `$HOME/.local/share/honkers-launcher/config.json`
pub fn config_file() -> anyhow::Result<PathBuf> {
    launcher_dir().map(|dir| dir.join("config.json"))
}
