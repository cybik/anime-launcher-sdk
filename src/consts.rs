use std::time::Duration;
use std::path::PathBuf;

use std::env;
use std::fs;

/// Timeout used by `anime_game_core::telemetry::is_disabled` to check acessibility of telemetry servers
pub const TELEMETRY_CHECK_TIMEOUT: Option<Duration> = Some(Duration::from_secs(3));

/// Timeout used by `anime_game_core::linux_patch::Patch::try_fetch` to fetch patch info
pub const PATCH_FETCHING_TIMEOUT: Option<Duration> = Some(Duration::from_secs(5));

/// Get default launcher dir path
/// 
/// `$HOME/.local/share/anime-game-launcher`
//#[inline]
pub fn launcher_dir() -> Option<PathBuf> {
    let configext = env::current_exe().ok().unwrap().parent().unwrap().join(".configext");
    let mut configname = String::from("anime-game-launcher");
    if configext.exists() {
        configname += &fs::read_to_string(configext).ok().unwrap();
    }
    dirs::data_dir().map(|dir| dir.join(configname))
}

/// Get default config file path
/// 
/// `$HOME/.local/share/anime-game-launcher/config.json`
#[inline]
pub fn config_file() -> Option<PathBuf> {
    launcher_dir().map(|dir| dir.join("config.json"))
}
