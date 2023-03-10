use steamlocate::*;
use std::env;
use std::fs;
use std::path::PathBuf;
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
            files: wine::Files {
                wine: "files/bin/wine".to_string(),
                wine64: Some("files/bin/wine64".to_string()),
                wineserver: Some("files/bin/wineserver".to_string()),
                winecfg: Some("files/lib64/wine/x86_64-windows/winecfg.exe".to_string()),
                wineboot: None // ehe
            },
            features: None,
            managed: true
        });
    }
    let mut wine_groups: Vec<wine::Group> = Vec::with_capacity(1);
    wine_groups.push(wine::Group {
        name:"steam-proton".to_string(),
        title:"Proton Runners via Steam".to_string(),
        features: wine::Features {
            need_dxvk: false,
            compact_launch: true,
            command: Some("python3 '%build%/proton' waitforexitandrun".to_string()),
            prefix_subdir: Some("pfx".to_string()),
            env: [
                ("STEAM_COMPAT_DATA_PATH".to_string(), "%prefix%".to_string()),
                ("STEAM_COMPAT_CLIENT_INSTALL_PATH".to_string(), "".to_string()),
                ("SteamAppId".to_string(), "0".to_string())
            ].iter().cloned().collect()
        },
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