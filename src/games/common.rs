use std::path::PathBuf;
use crate::integrations::steam;

/// Get assumed Steam Prefix install path
///
/// Generate a sane, possible, "relative to the prefix's C:\ root" install target for games that
///  need such a location to install the game(s) in.
pub fn base_install_dir(launcher_dir: PathBuf) -> anyhow::Result<PathBuf> {
    match steam::launched_from() {
        steam::LaunchedFrom::Independent => {
            Ok(launcher_dir)
        },
        steam::LaunchedFrom::Steam => {
            match steam::aagl_launcher_launch_target() {
                Some(target) => {
                    Ok(PathBuf::from(target).parent().unwrap().to_path_buf())
                },
                None => {
                    Ok(match steam::get_steam_compatdata_cdrive_root() {
                        Some(path) => PathBuf::from(path),
                        None => launcher_dir
                    })
                }
            }
        }
    }
}