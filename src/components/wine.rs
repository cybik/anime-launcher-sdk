use std::path::PathBuf;
use std::collections::HashMap;
use std::process::Output;
use std::io::Result;

use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;
use wincompatlib::prelude::*;

use super::loader::ComponentsLoader;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub title: String,
    pub features: Option<Features>,
    pub versions: Vec<Version>,
    pub managed: bool
}

impl Group {
    /// Find wine group with given name in components index
    /// 
    /// This method will also check all version names within this group, so both `wine-ge-proton` and `lutris-GE-Proton7-37-x86_64` will work
    pub fn find_in<T: Into<PathBuf>, F: AsRef<str>>(components: T, name: F) -> anyhow::Result<Option<Self>> {
        let name = name.as_ref();

        for group in get_groups(components)? {
            if group.name == name || group.versions.iter().any(move |version| version.name == name) {
                return Ok(Some(group));
            }
        }

        Ok(None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Features {
    pub bundle: Option<Bundle>,

    /// Whether this wine group needs DXVK
    pub need_dxvk: bool,

    /// Create temp bat file with `launcher.bat` call and its flags
    /// 
    /// Extremely helpful when your custom `command` feature can't handle multiline arguments (e.g. in GE-Proton)
    pub compact_launch: bool,

    /// Command used to launch the game
    /// 
    /// Available keywords:
    /// - `%build%` - path to wine build
    /// - `%prefix%` - path to wine prefix
    /// - `%temp%` - path to temp folder specified in config file
    /// - `%launcher%` - path to launcher folder
    /// - `%game%` - path to the game
    pub command: Option<String>,

    /// Standard environment variables that are applied when you launch the game
    /// 
    /// Available keywords:
    /// - `%build%` - path to wine build
    /// - `%prefix%` - path to wine prefix
    /// - `%temp%` - path to temp folder specified in config file
    /// - `%launcher%` - path to launcher folder
    /// - `%game%` - path to the game
    pub env: HashMap<String, String>,

    /// Managed prefix. Not set unless using the Steam variance.
    pub managed_prefix: Option<PathBuf>
}

impl Default for Features {
    fn default() -> Self {
        Self {
            bundle: None,
            need_dxvk: true,
            compact_launch: false,
            command: None,
            env: HashMap::new(),
            managed_prefix: None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Bundle {
    Proton
}

impl From<&JsonValue> for Features {
    fn from(value: &JsonValue) -> Self {
        let mut default = Self::default();

        Self {
            bundle: match value.get("bundle") {
                Some(value) => serde_json::from_value(value.to_owned()).unwrap_or(default.bundle),
                None => default.bundle
            },

            need_dxvk: match value.get("need_dxvk") {
                Some(value) => value.as_bool().unwrap_or(default.need_dxvk),
                None => default.need_dxvk
            },

            compact_launch: match value.get("compact_launch") {
                Some(value) => value.as_bool().unwrap_or(default.compact_launch),
                None => default.compact_launch
            },

            command: match value.get("command") {
                Some(value) => value.as_str().map(|value| value.to_string()),
                None => default.command
            },

            managed_prefix: None, // never set by the configuration.

            env: match value.get("env") {
                Some(value) => {
                    if let Some(object) = value.as_object() {
                        for (key, value) in object {
                            if let Some(value) = value.as_str() {
                                default.env.insert(key.to_string(), value.to_string());
                            } else {
                                default.env.insert(key.to_string(), value.to_string());
                            }
                        }
                    }

                    default.env
                }

                None => default.env
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    pub name: String,
    pub title: String,
    pub uri: String,
    pub files: Files,
    pub managed: bool,
    pub features: Option<Features>
}

impl Version {
    pub fn get_runner_dir<T: Into<PathBuf>>(&self, builds_dir: T) -> PathBuf {
        if self.managed {
            return self.uri.clone().into();
        }
        return builds_dir.into().join(&self.name);
    }

    /// Get latest recommended wine version
    pub fn latest<T: Into<PathBuf>>(components: T) -> anyhow::Result<Self> {
        Ok(get_groups(components)?[0].versions[0].clone())
    }

    /// Find wine version with given name in components index
    pub fn find_in<T: Into<PathBuf>, F: AsRef<str>>(components: T, name: F) -> anyhow::Result<Option<Self>> {
        let name = name.as_ref();

        for group in get_groups(components)? {
            if let Some(version) = group.versions.into_iter().find(move |version| version.name == name) {
                return Ok(Some(version));
            }
        }

        Ok(None)
    }

    /// Find wine group current version belongs to
    pub fn find_group<T: Into<PathBuf>>(&self, components: T) -> anyhow::Result<Option<Group>> {
        let name = self.name.as_str();

        for group in get_groups(components)? {
            if group.versions.iter().any(move |version| version.name == name) {
                return Ok(Some(group));
            }
        }

        Ok(None)
    }

    /// Check is current wine downloaded in specified folder
    #[inline]
    pub fn is_downloaded_in<T: Into<PathBuf>>(&self, folder: T) -> bool {
        folder.into().join(&self.name).exists()
    }

    /// Return this version's features
    #[inline]
    pub fn version_features(&self) -> Option<Features> {
        self.features.clone()
    }

    /// Return this version's features if they persist, or
    /// return group's features otherwise
    pub fn features_in(&self, group: &Group) -> Option<Features> {
        if self.features.is_some() {
            self.features.clone()
        }

        else {
            group.features.as_ref().map(|features| features.to_owned())
        }
    }

    /// Return this version's features if they persist, or
    /// try to return group's features otherwise
    pub fn features<T: Into<PathBuf>>(&self, components: T) -> anyhow::Result<Option<Features>> {
        if self.features.is_some() {
            Ok(self.features.clone())
        }

        else {
            match self.find_group(components)? {
                Some(group) => Ok(group.features),
                None => Ok(None)
            }
        }
    }

    /// Convert current wine struct to one from `wincompatlib`
    /// 
    /// `wine_folder` should point to the folder with wine binaries, so e.g. `/path/to/runners/wine-proton-ge-7.11`
    #[inline]
    pub fn to_wine<T: Into<PathBuf>>(&self, components: T, wine_folder: Option<T>) -> WincompatlibWine {
        let mut wine_folder = wine_folder.map(|folder| folder.into()).unwrap_or_default();
        if self.managed {
            wine_folder = PathBuf::from(&self.uri); // known case: if the proton install is managed, the URI is actually the local install.
        }

        let (wine, arch) = match self.files.wine64.as_ref() {
            Some(wine) => (wine, WineArch::Win64),
            None => (&self.files.wine, WineArch::Win32)
        };

        let wineboot = self.files.wineboot.as_ref().map(|wineboot| {
            let wineboot = PathBuf::from(wineboot);

            if let Some(ext) = wineboot.extension() {
                if ext == "exe" {
                    return WineBoot::Windows(wine_folder.join(wineboot));
                }
            }

            WineBoot::Unix(wine_folder.join(wineboot))
        });

        let wineserver = self.files.wineserver.as_ref().map(|wineserver| wine_folder.join(wineserver));

        if let Ok(Some(features)) = self.features(components) {
            if let Some(Bundle::Proton) = features.bundle {
                return WincompatlibWine::Proton(Proton::new(wine_folder, features.managed_prefix));
            }
        }

        WincompatlibWine::Default(Wine::new(
            wine_folder.join(wine),
            None,
            Some(arch),
            wineboot,
            wineserver,
            WineLoader::Current
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Files {
    pub wine: String,
    pub wine64: Option<String>,
    pub wineserver: Option<String>,
    pub wineboot: Option<String>,
    pub winecfg: Option<String>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WincompatlibWine {
    Default(Wine),
    Proton(Proton)
}

impl From<Wine> for WincompatlibWine {
    fn from(wine: Wine) -> Self {
        Self::Default(wine)
    }
}

impl From<Proton> for WincompatlibWine {
    fn from(proton: Proton) -> Self {
        Self::Proton(proton)
    }
}

impl WineWithExt for WincompatlibWine {
    fn with_prefix<T: Into<PathBuf>>(self, prefix: T) -> Self {
        match self {
            Self::Default(wine) => Self::Default(wine.with_prefix(prefix)),
            Self::Proton(proton) => Self::Proton(proton.with_prefix(prefix))
        }
    }

    fn with_arch(self, arch: WineArch) -> Self {
        match self {
            Self::Default(wine) => Self::Default(wine.with_arch(arch)),
            Self::Proton(proton) => Self::Proton(proton.with_arch(arch))
        }
    }

    fn with_boot(self, boot: WineBoot) -> Self {
        match self {
            Self::Default(wine) => Self::Default(wine.with_boot(boot)),
            Self::Proton(proton) => Self::Proton(proton.with_boot(boot))
        }
    }

    fn with_server<T: Into<PathBuf>>(self, server: T) -> Self {
        match self {
            Self::Default(wine) => Self::Default(wine.with_server(server)),
            Self::Proton(proton) => Self::Proton(proton.with_server(server))
        }
    }

    fn with_loader(self, loader: WineLoader) -> Self {
        match self {
            Self::Default(wine) => Self::Default(wine.with_loader(loader)),
            Self::Proton(proton) => Self::Proton(proton.with_loader(loader))
        }
    }
}

impl WineBootExt for WincompatlibWine {
    fn wineboot_command(&self) -> std::process::Command {
        match self {
            Self::Default(wine) => wine.wineboot_command(),
            Self::Proton(proton) => proton.wineboot_command()
        }
    }

    fn update_prefix<T: Into<PathBuf>>(&self, path: Option<T>) -> Result<Output> {
        match self {
            Self::Default(wine) => wine.update_prefix(path),
            Self::Proton(proton) => proton.update_prefix(path)
        }
    }

    fn stop_processes(&self, force: bool) -> Result<Output> {
        match self {
            Self::Default(wine) => wine.stop_processes(force),
            Self::Proton(proton) => proton.stop_processes(force)
        }
    }

    fn restart(&self) -> Result<Output> {
        match self {
            Self::Default(wine) => wine.restart(),
            Self::Proton(proton) => proton.restart()
        }
    }

    fn shutdown(&self) -> Result<Output> {
        match self {
            Self::Default(wine) => wine.shutdown(),
            Self::Proton(proton) => proton.shutdown()
        }
    }

    fn end_session(&self) -> Result<Output> {
        match self {
            Self::Default(wine) => wine.end_session(),
            Self::Proton(proton) => proton.end_session()
        }
    }
}

pub fn get_groups<T: Into<PathBuf>>(components: T) -> anyhow::Result<Vec<Group>> {
    ComponentsLoader::new(components).get_wine_versions()
}

/// List downloaded wine versions in some specific folder
pub fn get_downloaded<T: Into<PathBuf>>(components: T, folder: T) -> anyhow::Result<Vec<Group>> {
    let mut downloaded = Vec::new();

    let folder: PathBuf = folder.into();

    for mut group in get_groups(components)? {
        if group.managed {
            // special case: Runners are externally managed.
            downloaded.push(group);
            continue;
        }
        group.versions.retain(|version| folder.join(&version.name).exists());

        if !group.versions.is_empty() {
            downloaded.push(group);
        }
    }

    Ok(downloaded)
}
