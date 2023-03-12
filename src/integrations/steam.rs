use steamlocate::*;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use crate::components::wine;

pub fn launched_from_steam() -> bool {
    match env::var_os("SteamEnv") {
        Some(val) => return val == "1",
        None => return false
    };
}

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

fn filter_local_roots_by_proton_launcher() -> Vec<PathBuf> {
    let mut _processed: Vec<PathBuf> = Vec::new();
    for _local in get_local_search_roots() {
        if _local.is_dir() {
            for _ld in _local.read_dir().unwrap() {
                let _pld = PathBuf::from(_ld.unwrap().path());
                if _pld.is_dir() 
                    && !_pld.is_symlink() 
                    && _pld.join("proton").exists()
                {
                    _processed.push(_pld.clone())
                }
            }
        }
    }
    return _processed;
}

pub fn get_proton_installs_as_wines() -> anyhow::Result<Vec<wine::Group>> {
    let mut wines: Vec<wine::Version> = Vec::new();
    let mut these_features = wine::Features::default();
    these_features.need_dxvk = false;
    these_features.compact_launch = true;
    these_features.command = Some("python3 '%build%/proton' waitforexitandrun".to_string());
    these_features.prefix_subdir = Some("pfx".to_string());
    let this_env: HashMap<String, String> = [
        ("STEAM_COMPAT_DATA_PATH".to_string(), "%prefix%".to_string()),
        ("STEAM_COMPAT_CLIENT_INSTALL_PATH".to_string(), "".to_string()),
        ("SteamAppId".to_string(), "0".to_string())
    ].iter().cloned().collect();
    these_features.env = this_env;
    let mut these_files = wine::Files::default();
    these_files.wine = "files/bin/wine".to_string();
    these_files.wine64 = Some("files/bin/wine64".to_string());
    these_files.wineserver = Some("files/bin/wineserver".to_string());
    these_files.winecfg = Some("files/lib64/wine/x86_64-windows/winecfg.exe".to_string());

    for path in filter_local_roots_by_proton_launcher() {
        let version_file = fs::read_to_string(path.join("version"))
            .expect("Should have been able to read the file");
        let split : Vec<&str> = version_file.split(" ").collect();
        if split.len() < 2 {
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
        tracing::debug!("Proton Wine Version Info :: path {0} :: name {1} :: title {2}", (&path.to_str().unwrap()).to_string(), split[1], name);
        wines.push(wine::Version {
            name: split.get(1).expect("Should really be set right now").to_string(),   // clarify
            title: name.clone(),  // clarify
            uri: (&path.to_str().unwrap()).to_string(), // clarify
            files: these_files.clone(),// massively clarify lol
            features: None
        });
    }
    let mut wine_groups: Vec<wine::Group> = Vec::with_capacity(1);
    wine_groups.push(wine::Group {
        name:"steam-proton".to_string(),
        title:"Steam Proton".to_string(),
        features: these_features,
        versions: wines,
        managed: true // will be changed.
    });
    Ok(wine_groups)
}

pub fn steam_proton_installed_paths() -> Option<Vec<PathBuf>> {
    if !launched_from_steam() {
        None
    } else if SteamDir::locate().is_none() {
        None
    } else {
        Some(filter_local_roots_by_proton_launcher())
    }
}