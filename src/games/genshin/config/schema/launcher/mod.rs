use std::path::PathBuf;

use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

use enum_ordinalize::Ordinalize;

use anime_game_core::genshin::consts::GameEdition;

use crate::config::schema_blanks::prelude::*;
use crate::genshin::consts::launcher_dir;

#[cfg(feature = "environment-emulation")]
use crate::genshin::env_emulation::Environment;

#[cfg(feature = "discord-rpc")]
pub mod discord_rpc;

pub mod prelude {
    pub use super::{
        Launcher,
        LauncherStyle,
        LauncherBehavior
    };

    #[cfg(feature = "discord-rpc")]
    pub use super::discord_rpc::DiscordRpc;
}

use prelude::*;
use crate::integrations::steam;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ordinalize, Serialize, Deserialize)]
pub enum LauncherStyle {
    Modern,
    Classic
}

impl Default for LauncherStyle {
    #[inline]
    fn default() -> Self {
        Self::Modern
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ordinalize, Serialize, Deserialize)]
pub enum LauncherBehavior {
    Nothing,
    Hide,
    Close
}

impl Default for LauncherBehavior {
    #[inline]
    fn default() -> Self {
        Self::Hide
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Launcher {
    pub language: String,
    pub edition: GameEdition,
    pub style: LauncherStyle,
    pub temp: Option<PathBuf>,
    pub repairer: Repairer,

    pub permissive: bool,

    #[cfg(feature = "discord-rpc")]
    pub discord_rpc: DiscordRpc,

    #[cfg(feature = "environment-emulation")]
    pub environment: Environment,

    pub behavior: LauncherBehavior
}

impl Default for Launcher {
    #[inline]
    fn default() -> Self {
        Self {
            language: String::from("en-us"),
            edition: GameEdition::from_system_lang(),
            style: LauncherStyle::default(),
            temp: launcher_dir().ok(),
            repairer: Repairer::default(),

            permissive: steam::launched_from_steam(),

            #[cfg(feature = "discord-rpc")]
            discord_rpc: DiscordRpc::default(),

            #[cfg(feature = "environment-emulation")]
            environment: Environment::default(),

            behavior: LauncherBehavior::default()
        }
    }
}

impl From<&JsonValue> for Launcher {
    fn from(value: &JsonValue) -> Self {
        let default = Self::default();

        Self {
            language: match value.get("language") {
                Some(value) => value.as_str().unwrap_or(&default.language).to_string(),
                None => default.language
            },

            edition: match value.get("edition") {
                Some(value) => serde_json::from_value(value.clone()).unwrap_or(default.edition),
                None => default.edition
            },

            style: match value.get("style") {
                Some(value) => serde_json::from_value(value.to_owned()).unwrap_or_default(),
                None => default.style
            },

            temp: match value.get("temp") {
                Some(value) => {
                    if value.is_null() {
                        None
                    } else {
                        match value.as_str() {
                            Some(value) => Some(PathBuf::from(value)),
                            None => default.temp
                        }
                    }
                },
                None => default.temp
            },

            repairer: match value.get("repairer") {
                Some(value) => Repairer::from(value),
                None => default.repairer
            },

            permissive: match value.get("permissive") {
                Some(value) => serde_json::from_value(value.to_owned()).unwrap_or_default(),
                None => false
            },

            #[cfg(feature = "discord-rpc")]
            discord_rpc: match value.get("discord_rpc") {
                Some(value) => DiscordRpc::from(value),
                None => default.discord_rpc
            },

            #[cfg(feature = "environment-emulation")]
            environment: match value.get("environment") {
                Some(value) => serde_json::from_value(value.clone()).unwrap_or(default.environment),
                None => default.environment
            },

            behavior: match value.get("behavior") {
                Some(value) => serde_json::from_value(value.clone()).unwrap_or(default.behavior),
                None => default.behavior
            }
        }
    }
}
