use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

use anime_game_core::genshin::consts::GameEdition;

use crate::genshin::consts::launcher_dir;
use crate::integrations::steam;

use crate::genshin::consts::base_game_install_dir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paths {
    pub global: PathBuf,
    pub china: PathBuf
}

impl Paths {
    #[inline]
    /// Get game path for given edition
    pub fn for_edition(&self, edition: impl Into<GameEdition>) -> &Path {
        match edition.into() {
            GameEdition::Global => self.global.as_path(),
            GameEdition::China => self.china.as_path()
        }
    }
}

fn concat_gen_shin() -> String {
    format!("{}{}{} {}{}","Ge", "nshi", "n", "Imp", "act")
}

fn concat_gen_shin_game() -> String {
    format!("{} {}", concat_gen_shin(), "game")
}

impl Default for Paths {
    fn default() -> Self {
        let launcher_dir = base_game_install_dir().expect("Failed to get launcher dir");

        Self {
            global: match steam::launched_from() {
                steam::LaunchedFrom::Independent => launcher_dir.join(concat_gen_shin()),
                steam::LaunchedFrom::Steam => launcher_dir.join(concat_gen_shin()).join(concat_gen_shin_game()),
            },
            china: launcher_dir.join(concat!("Yu", "anS", "hen")) // TODO: autogen Steam
        }
    }
}

impl From<&JsonValue> for Paths {
    fn from(value: &JsonValue) -> Self {
        let default = Self::default();

        // SDK 0.5.11 (launcher 3.3.0) and earlier
        if value.is_string() {
            let path = PathBuf::from(value.as_str().unwrap());

            Self {
                china: match path.parent() {
                    Some(parent) => parent.join(concat!("Yu", "anS", "hen")),
                    None => default.china
                },
                global: path
            }
        }

        // SDK 0.5.12 and later
        else {
            Self {
                global: match value.get("global") {
                    Some(value) => match value.as_str() {
                        Some(value) => PathBuf::from(value),
                        None => default.global
                    },
                    None => default.global
                },
    
                china: match value.get("china") {
                    Some(value) => match value.as_str() {
                        Some(value) => PathBuf::from(value),
                        None => default.china
                    },
                    None => default.china
                }
            }
        }
    }
}
