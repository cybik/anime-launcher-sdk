use std::path::PathBuf;

use anime_game_core::prelude::*;
use anime_game_core::genshin::prelude::*;

use crate::config::ConfigExt;
use crate::genshin::config::Config;

#[derive(Debug, Clone)]
pub enum LauncherState {
    Launch,

    /// Always contains `VersionDiff::Predownload`
    PredownloadAvailable {
        game: VersionDiff,
        voices: Vec<VersionDiff>
    },

    FolderMigrationRequired {
        from: PathBuf,
        to: PathBuf,
        cleanup_folder: Option<PathBuf>
    },

    PlayerPatchAvailable {
        patch: PlayerPatch,
        disable_mhypbase: bool
    },

    TelemetryNotDisabled,

    #[cfg(feature = "components")]
    WineNotInstalled,

    PrefixNotExists,

    // Always contains `VersionDiff::Diff`
    VoiceUpdateAvailable(VersionDiff),

    /// Always contains `VersionDiff::Outdated`
    VoiceOutdated(VersionDiff),

    /// Always contains `VersionDiff::NotInstalled`
    VoiceNotInstalled(VersionDiff),

    // Always contains `VersionDiff::Diff`
    GameUpdateAvailable(VersionDiff),

    /// Always contains `VersionDiff::Outdated`
    GameOutdated(VersionDiff),

    /// Always contains `VersionDiff::NotInstalled`
    GameNotInstalled(VersionDiff)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateUpdating {
    Game,
    Voice(VoiceLocale),
    Patch
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherStateParams<F: Fn(StateUpdating)> {
    pub game_path: PathBuf,
    pub game_edition: GameEdition,

    pub wine_prefix: PathBuf,
    pub selected_voices: Vec<VoiceLocale>,

    pub patch_servers: Vec<String>,
    pub patch_folder: PathBuf,
    pub use_patch: bool,
    pub disable_mhypbase: bool,

    pub status_updater: F
}

impl LauncherState {
    pub fn get<F: Fn(StateUpdating)>(params: LauncherStateParams<F>) -> anyhow::Result<Self> {
        tracing::debug!("Trying to get launcher state");
        let mut managed = false;
        let config = Config::get()?;

        if let Some(wine) = config.get_selected_wine()? {
            managed = wine.managed;
        }
        // Check prefix existence
        if !params.wine_prefix.join("drive_c").exists() && !managed {
            return Ok(Self::PrefixNotExists);
        }

        // Check game installation status
        (params.status_updater)(StateUpdating::Game);

        let game = Game::new(&params.game_path, params.game_edition);

        let diff = game.try_get_diff()?;

        match diff {
            VersionDiff::Latest { .. } | VersionDiff::Predownload { .. } => {
                let mut predownload_voice = Vec::new();

                for locale in params.selected_voices {
                    let mut voice_package = VoicePackage::with_locale(locale, params.game_edition)?;

                    (params.status_updater)(StateUpdating::Voice(voice_package.locale()));

                    // Replace voice package struct with the one constructed in the game's folder
                    // so it'll properly calculate its difference instead of saying "not installed"
                    if voice_package.is_installed_in(&params.game_path) {
                        voice_package = match VoicePackage::new(get_voice_package_path(&params.game_path, params.game_edition, voice_package.locale()), params.game_edition) {
                            Some(locale) => locale,
                            None => return Err(anyhow::anyhow!("Failed to load {} voice package", voice_package.locale().to_name()))
                        };
                    }

                    let diff = voice_package.try_get_diff()?;

                    match diff {
                        VersionDiff::Latest { .. } => (),
                        VersionDiff::Predownload { .. } => predownload_voice.push(diff),

                        VersionDiff::Diff { .. } => return Ok(Self::VoiceUpdateAvailable(diff)),
                        VersionDiff::Outdated { .. } => return Ok(Self::VoiceOutdated(diff)),
                        VersionDiff::NotInstalled { .. } => return Ok(Self::VoiceNotInstalled(diff))
                    }
                }

                // Check game patch status
                (params.status_updater)(StateUpdating::Patch);

                let patch = Patch::new(&params.patch_folder, params.game_edition);

                // Sync local patch folder with remote if needed
                // TODO: maybe I shouldn't do it here?
                if patch.is_sync(&params.patch_servers)?.is_none() {
                    for server in &params.patch_servers {
                        if patch.sync(server).is_ok() {
                            break;
                        }
                    }
                }

                // Check UnityPlayer patch
                if params.use_patch {
                    let player_patch = patch.player_patch()?;

                    if !player_patch.is_applied(&params.game_path)? {
                        return Ok(Self::PlayerPatchAvailable {
                            patch: player_patch,
                            disable_mhypbase: params.disable_mhypbase
                        });
                    }
                }

                // Check telemetry servers
                let disabled = telemetry::is_disabled(params.game_edition)

                    // Return true if there's no domain name resolved, or false otherwise
                    .map(|result| result.is_none())

                    // And return true if there's an error happened during domain name resolving
                    // FIXME: might not be a good idea? Idk
                    .unwrap_or_else(|err| {
                        tracing::warn!("Failed to check telemetry servers: {err}. Assuming they're disabled");

                        true
                    });

                if !disabled {
                    return Ok(Self::TelemetryNotDisabled);
                }

                // Check if update predownload available
                if let VersionDiff::Predownload { .. } = diff {
                    Ok(Self::PredownloadAvailable {
                        game: diff,
                        voices: predownload_voice
                    })
                }

                // Otherwise we can launch the game
                else {
                    Ok(Self::Launch)
                }
            }

            VersionDiff::Diff { .. } => Ok(Self::GameUpdateAvailable(diff)),
            VersionDiff::Outdated { .. } => Ok(Self::GameOutdated(diff)),
            VersionDiff::NotInstalled { .. } => Ok(Self::GameNotInstalled(diff))
        }
    }

    #[cfg(feature = "config")]
    #[tracing::instrument(level = "debug", skip(status_updater), ret)]
    pub fn get_from_config<T: Fn(StateUpdating)>(status_updater: T) -> anyhow::Result<Self> {
        tracing::debug!("Trying to get launcher state");

        let config = Config::get()?;

        match &config.game.wine.selected {
            #[cfg(feature = "components")]
            Some(selected) if (!config.game.wine.builds.join(selected).exists() && !selected.managed) => return Ok(Self::WineNotInstalled),
            None => return Ok(Self::WineNotInstalled),
            _ => ()
        }

        let mut voices = Vec::with_capacity(config.game.voices.len());

        for voice in &config.game.voices {
            voices.push(match VoiceLocale::from_str(voice) {
                Some(locale) => locale,
                None => return Err(anyhow::anyhow!("Incorrect voice locale \"{}\" specified in the config", voice))
            });
        }

        Self::get(LauncherStateParams {
            game_path: config.game.path.for_edition(config.launcher.edition).to_path_buf(),
            game_edition: config.launcher.edition,

            wine_prefix: config.get_wine_prefix_path(),
            selected_voices: voices,

            patch_servers: config.patch.servers,
            patch_folder: config.patch.path,
            use_patch: config.patch.apply,
            disable_mhypbase: config.patch.disable_mhypbase,

            status_updater
        })
    }
}
