use std::path::PathBuf;

use std::env;
use std::fs;

/// Timeout used by `anime_game_core::telemetry::is_disabled` to check acessibility of telemetry servers
pub const TELEMETRY_CHECK_TIMEOUT: Option<u64> = Some(3);

/// Get default launcher dir path
/// 
/// `$HOME/.local/share/anime-game-launcher`
//#[inline]
pub fn launcher_dir() -> anyhow::Result<PathBuf> {
    let configext = env::current_exe().ok().unwrap().parent().unwrap().join(".configext");
    let mut configname = String::from("anime-game-launcher");
    if configext.exists() {
        configname += &fs::read_to_string(configext).ok().unwrap();
    }
    Ok(dirs::data_dir().map(|dir| dir.join(configname)))
}

/// Get default config file path
/// 
/// `$HOME/.local/share/anime-game-launcher/config.json`
#[inline]
pub fn config_file() -> anyhow::Result<PathBuf> {
    launcher_dir().map(|dir| dir.join("config.json"))
}
