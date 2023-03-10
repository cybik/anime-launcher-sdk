use steamlocate::*;
use std::env;
use std::fs;
use std::path::PathBuf;
use crate::components;
use crate::components::wine;

/// Identify whether we were launched through a Steam environment.
pub fn launched_from_steam() -> bool {
    match env::var_os("SteamEnv") {
        Some(val) => return val == "1",
        None => return false
    };
}

/// Identify whether we are running on Steam Deck.
pub fn is_steam_deck() -> bool {
    match env::var_os("SteamDeck") {
        Some(val) => return val == "1",
        None => return false
    };
}

/// Identify whether we were launched through a SteamOS environment.
pub fn is_steam_os() -> bool {
    match env::var_os("SteamOS") {
        Some(val) => return val == "1",
        None => return false
    };
}

/// Generate a list of Steam-inventoried search roots.
fn get_local_search_roots() -> Vec<PathBuf> {
    // initialize and let Steam seed itself.
    let mut _steamdir : SteamDir = SteamDir::locate().unwrap();
    _steamdir.libraryfolders();

    let mut _vec_steam_managed_protons : Vec<PathBuf> = Vec::new();
    _vec_steam_managed_protons.push(_steamdir.path.clone().join("compatibilitytools.d"));
    for _path in &_steamdir.libraryfolders().paths {
        _vec_steam_managed_protons.push(_path.clone().join("common"));
    }
    return _vec_steam_managed_protons;
}

/// Inventory all possible Proton launchers in search roots.
fn filter_local_roots_by_proton_launcher() -> Vec<PathBuf> {
    let mut _processed: Vec<PathBuf> = Vec::new();
    for _local in get_local_search_roots() {
        if _local.exists() && _local.is_dir() {
            for _ld in _local.read_dir().unwrap() {
                let _pld = PathBuf::from(_ld.unwrap().path());
                if _pld.is_dir() // is it a directory that contains things
                    && !_pld.is_symlink() // is it NOT a symlink (don't inventory dopplegangers)
                    && _pld.join("proton").exists() // does the directory contain a proton launch script/file?
                {
                    _processed.push(_pld.clone()) // aye, we got a culprit
                }
            }
        }
    }
    return _processed;
}

/// Generate a list of WinCompatLib Structs for inventoried Steam-managed, detected Proton installs
pub fn get_proton_installs_as_wines() -> anyhow::Result<Vec<wine::Group>> {
    let mut wines: Vec<wine::Version> = Vec::new();

    let mut proton_features = components::wine::Features::default();
    proton_features.bundle = Some(components::wine::Bundle::Proton);
    proton_features.compact_launch = true;
    match env::var_os("STEAM_COMPAT_DATA_PATH") {
        Some(val) => {
            tracing::debug!("MAYBE HAZ? {0}", val.to_str().unwrap());
            proton_features.managed_prefix = Some(PathBuf::from(val));
        },
        None => {}
    };
    for path in filter_local_roots_by_proton_launcher() {
        let version_file = fs::read_to_string(path.join("version"))
            .expect("Should have been able to read the file");
        let split : Vec<&str> = version_file.split(" ").collect();
        if split.len() < 2 {
            // oof.
            tracing::debug!("Proton at {0} is so old the version file doesn't follow spec. Skipping.", (&path.to_str().unwrap()).to_string());
            continue;
        }

        let name = match path.file_name() {
            Some(file_path) => match file_path.to_str(){
                Some(path_name) => path_name.to_string(),
                None => anyhow::bail!("Bad file entry somehow")
            },
            None => anyhow::bail!("Bad file entry somehow")
        };

        // Let's gooooo!
        wines.push(wine::Version {
            name: split.get(1).expect("Should really be set right now").trim().to_string(),   // clarify
            title: name.clone().trim().to_string(),  // clarify
            uri: (&path.to_str().unwrap()).trim().to_string(), // clarify
            files: wine::Files { // handled by wincompatlib
                wine: "proton".to_string(),
                wine64: None,
                wineserver: None,
                wineboot: None
            },
            features: Some(proton_features.clone()), // handled
            managed: true
        });
    }
    let mut wine_groups: Vec<wine::Group> = Vec::with_capacity(1);
    wine_groups.push(wine::Group {
        name:"steam-proton".to_string(),
        title:"Proton Runners via Steam".to_string(),
        features: Some(proton_features.clone()), // handled
        versions: wines,
        managed: true
    });
    Ok(wine_groups)
}

/// Get a list of Proton paths to sleuth into.
pub fn steam_proton_installed_paths() -> Option<Vec<PathBuf>> {
    if !launched_from_steam() {
        None
    } else if SteamDir::locate().is_none() {
        None
    } else {
        Some(filter_local_roots_by_proton_launcher())
    }
}