use steamlocate::*;
use std::env;
use std::fs;
use std::path::PathBuf;
use crate::components;

#[derive(Debug, Clone, PartialEq)]
pub enum LaunchedFrom {
    Steam,
    // TODO: Heroic?
    Independent
}

#[derive(Debug, Clone, PartialEq)]
pub enum Steam {
    Desktop,
    Deck,
    OS,
    /// ...
    Invalid
}

pub fn environment() -> Steam {
    match launched_from_steam() {
        true => match is_steam_os() {
            true => match is_steam_deck() {
                true => Steam::Deck,
                false => Steam::OS
            },
            false => Steam::Desktop
        },
        false => Steam::Invalid
    }
}

pub fn launched_from() -> LaunchedFrom {
    if environment() == Steam::Invalid {
        return LaunchedFrom::Independent;
    }
    LaunchedFrom::Steam
}

/// Identify whether we were launched through a Steam environment.
fn launched_from_steam() -> bool {
    match env::var_os("SteamEnv") {
        Some(val) => val == "1",
        None => false
    }
}

/// Identify whether we are running on Steam Deck.
fn is_steam_deck() -> bool {
    match env::var_os("SteamDeck") {
        Some(val) => val == "1",
        None => false
    }
}

/// Identify whether we were launched through a SteamOS environment.
fn is_steam_os() -> bool {
    match env::var_os("SteamOS") {
        Some(val) => val == "1",
        None => false
    }
}

/// Prefix updates are disabled on Steam, as we assume the runners are Proton-spec and manage that.
pub fn is_prefix_update_disabled() -> bool {
    launched_from() == LaunchedFrom::Steam
}

pub fn default_window_size_width(default: i32) -> i32 {
    match is_steam_deck() {
        true => 1280,
        false => default
    }
}

pub fn default_window_size_height(default: i32) -> i32 {
    match is_steam_deck() {
        true => 800,
        false => default
    }
}
/// Generate a list of Steam-inventoried search roots.
fn get_library_search_roots() -> Option<Vec<PathBuf>> {
    // initialize and let Steam seed itself.
    match SteamDir::locate() {
        Some(mut steam_install_dir) => {
            Some(steam_install_dir.libraryfolders().paths
                .clone().into_iter()
                .map(|single_path| single_path.join("common"))
                .collect::<Vec<PathBuf>>())
        }
        None => None
    }
}

fn get_homedir_search_roots() -> Option<PathBuf> {
    match SteamDir::locate() {
        Some(steam_install_dir) => {
            Some(steam_install_dir.path.clone().join("compatibilitytools.d"))
        }
        None => None
    }
}

fn check_pld(_ld: PathBuf) -> Option<PathBuf> {
    let _pld = PathBuf::from(_ld);
    match _pld.is_dir() // is it a directory that contains things
            && !_pld.is_symlink() // is it NOT a symlink (don't inventory dopplegangers)
            && _pld.join("proton").exists() // does the directory contain a proton launch script/file?
    {
        true => Some(_pld),
        false => None
    }
}

fn check_root(_local: PathBuf) -> Option<Vec<PathBuf>> {
    let mut _processed: Vec<PathBuf> = Vec::new();
    if _local.exists() && _local.is_dir() {
        for _ld in _local.read_dir().unwrap() {
            match check_pld(_ld.unwrap().path()) {
                Some(_pld) => _processed.push(_pld),
                None => {}
            }
        }
    }
    Some(_processed)
}

/// Inventory all possible Proton launchers in search roots.
fn filter_local_roots_by_proton_launcher() -> Option<Vec<PathBuf>> {
    let mut _processed: Vec<PathBuf> = Vec::new();
    match get_homedir_search_roots() {
        None => {},
        Some(_local) => match check_root(_local) {
            Some(_root) => _processed.extend(_root),
            None => {}
        }
    }
    match get_library_search_roots() {
        None => { },
        Some(_locals) => {
            for _local in _locals {
                match check_root(_local) {
                    Some(_root) => _processed.extend(_root),
                    None => {}
                }
            }
        }
    }
    Some(_processed)
}

/// Generate a list of WinCompatLib Structs for inventoried Steam-managed, detected Proton installs
pub fn get_proton_installs_as_wines() -> anyhow::Result<Vec<components::wine::Group>> {
    match filter_local_roots_by_proton_launcher() {
        Some(paths) => {
            let proton_features = components::wine::Features {
                bundle: Some(components::wine::Bundle::Proton),
                compact_launch: true,
                command: Some(String::from("python3 '%build%/proton' waitforexitandrun")),
                managed_prefix: match env::var_os("STEAM_COMPAT_DATA_PATH") {
                    Some(val) => {
                        tracing::debug!("MAYBE HAZ? {0}", val.to_str().unwrap());
                        Some(PathBuf::from(val))
                    },
                    None => None
                },
                ..components::wine::Features::default()
            };
            let mut wines: Vec<components::wine::Version> = Vec::new();
            for path in paths {
                let version_file = fs::read_to_string(path.join("version")).expect(
                format!("Should have been able to read the file for {0}",
                    path.display()).as_str()
                );

                let split : Vec<&str> = version_file.split(" ").collect();
                if split.len() < 2 { continue; } // Proton so old the version file broke spec.

                let name = match path.file_name() {
                    Some(file_path) => match file_path.to_str(){
                        Some(path_name) => path_name.to_string(),
                        None => anyhow::bail!("Bad file entry somehow")
                    },
                    None => anyhow::bail!("Bad file entry somehow")
                };

                // Let's gooooo!
                wines.push(components::wine::Version {
                    name: split.get(1).expect("Should really be set right now").trim().to_string(),   // clarify
                    title: name.clone().trim().to_string(),  // clarify
                    uri: (&path.to_str().unwrap()).trim().to_string(), // clarify
                    format: None,
                    files: components::wine::Files { // handled by wincompatlib
                        wine: "proton".to_string(),
                        wine64: None,
                        wineserver: None,
                        wineboot: None
                    },
                    features: Some(proton_features.clone()), // handled
                    managed: true
                });
            }
            Ok([
                components::wine::Group {
                    name:"steam-proton".to_string(),
                    title:"Proton Runners via Steam".to_string(),
                    features: Some(proton_features.clone()), // handled
                    versions: wines,
                    managed: true
                }
            ].to_vec())
        },
        None => Err(anyhow::anyhow!("Steam mode active but no roots?"))
    }
}

/// Get a list of Proton paths to sleuth into.
pub fn steam_proton_installed_paths() -> Option<Vec<PathBuf>> {
    match !launched_from_steam() || SteamDir::locate().is_none() {
        true => None,
        false => filter_local_roots_by_proton_launcher()
    }
}