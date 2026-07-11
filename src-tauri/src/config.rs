//! Configuration management module for PTD Launcher.
//! Handles loading/saving config, settings, and version information.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Flash player configuration per OS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashPlayerOs {
    pub primary_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_url: Option<String>,
    pub filename: String,
}

/// Flash player configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashPlayerConfig {
    pub fallback_version: String,
    pub windows: FlashPlayerOs,
    pub macos: FlashPlayerOs,
    pub linux: FlashPlayerOs,
}

impl Default for FlashPlayerConfig {
    fn default() -> Self {
        Self {
            fallback_version: "32.0.0.465".to_string(),
            windows: FlashPlayerOs {
                primary_url: "https://www.flash.cn/cdm/latest/flashplayer_sa.exe".to_string(),
                fallback_url: Some("https://fpdownload.macromedia.com/pub/flashplayer/updaters/32/flashplayer_32_sa.exe".to_string()),
                filename: "flashplayer_sa.exe".to_string(),
            },
            macos: FlashPlayerOs {
                primary_url: "https://fpdownload.macromedia.com/pub/flashplayer/updaters/32/flashplayer_32_sa.dmg".to_string(),
                fallback_url: None,
                filename: "Flash Player.app".to_string(),
            },
            linux: FlashPlayerOs {
                primary_url: "https://fpdownload.macromedia.com/pub/flashplayer/updaters/32/flash_player_sa_linux.x86_64.tar.gz".to_string(),
                fallback_url: Some("https://archive.org/download/flashplayer_standalone_projectors/flash_player_sa_linux.x86_64.tar.gz".to_string()),
                filename: "flashplayer".to_string(),
            },
        }
    }
}

/// Ruffle configuration per OS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuffleOs {
    pub url: String,
    pub filename: String,
}

/// Ruffle configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuffleConfig {
    pub windows: RuffleOs,
    pub macos: RuffleOs,
    pub linux: RuffleOs,
}

impl Default for RuffleConfig {
    fn default() -> Self {
        Self {
            // These are fallback URLs used if fetching the latest nightly fails
            windows: RuffleOs {
                url: "https://github.com/ruffle-rs/ruffle/releases/download/v0.3.0/ruffle-0.3.0-windows-x86_64.zip".to_string(),
                filename: "ruffle.exe".to_string(),
            },
            macos: RuffleOs {
                url: "https://github.com/ruffle-rs/ruffle/releases/download/v0.3.0/ruffle-0.3.0-macos-universal.tar.gz".to_string(),
                filename: "ruffle".to_string(),
            },
            linux: RuffleOs {
                url: "https://github.com/ruffle-rs/ruffle/releases/download/v0.3.0/ruffle-0.3.0-linux-x86_64.tar.gz".to_string(),
                filename: "ruffle".to_string(),
            },
        }
    }
}

/// Main application configuration (loaded from config.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub flash_player: FlashPlayerConfig,
    pub ruffle: RuffleConfig,
    pub game_urls: HashMap<String, String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            flash_player: FlashPlayerConfig::default(),
            ruffle: RuffleConfig::default(),
            game_urls: [
                (
                    "PTD1".to_string(),
                    "https://ptd.onl/ptd1-latest.swf".to_string(),
                ),
                (
                    "PTD1_Hacked".to_string(),
                    "https://ptd.onl/ptd1-hacked-latest.swf".to_string(),
                ),
                (
                    "PTD2".to_string(),
                    "https://ptd.onl/ptd2-latest.swf".to_string(),
                ),
                (
                    "PTD2_Hacked".to_string(),
                    "https://ptd.onl/ptd2-hacked-latest.swf".to_string(),
                ),
                (
                    "PTD3".to_string(),
                    "https://ptd.onl/ptd3-latest.swf".to_string(),
                ),
                (
                    "PTD3_Hacked".to_string(),
                    "https://ptd.onl/ptd3-hacked-latest.swf".to_string(),
                ),
            ]
            .into_iter()
            .collect(),
        }
    }
}

/// Version tracking for games and flash player
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameVersions {
    #[serde(default)]
    pub flash_player: String,
    #[serde(default)]
    pub ruffle: String,
    #[serde(default)]
    pub games: HashMap<String, String>,
    /// ISO 8601 timestamp of when Flash Player was last installed/updated
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub flash_installed_at: String,
    /// ISO 8601 timestamp of when Ruffle was last installed/updated
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ruffle_installed_at: String,
}

/// User settings (stored in settings.json)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flash_player_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_ruffle: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ruffle_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound_enabled: Option<bool>,
}

/// Get the application data directory based on OS
pub fn get_app_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(|p| PathBuf::from(p).join("PTD Launcher"))
            .map_err(|_| "Failed to get APPDATA".to_string())
    }

    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .map(|p| p.join("Library/Application Support/PTD Launcher"))
            .ok_or_else(|| "Failed to get home directory".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        dirs::home_dir()
            .map(|p| p.join(".local/share/PTD Launcher"))
            .ok_or_else(|| "Failed to get home directory".to_string())
    }
}

/// Get the games directory path
pub fn get_games_dir() -> Result<PathBuf, String> {
    get_app_dir().map(|p| p.join("Games"))
}

/// Get the flash player directory path
pub fn get_flash_dir() -> Result<PathBuf, String> {
    get_app_dir().map(|p| p.join("Flash"))
}

/// Get the ruffle directory path
pub fn get_ruffle_dir() -> Result<PathBuf, String> {
    get_app_dir().map(|p| p.join("Ruffle"))
}

/// Load the bundled config.json (app configuration)
pub fn load_config(config_path: &PathBuf) -> Result<AppConfig, String> {
    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read config.json: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse config.json: {}", e))
}

/// Load version information from version.json
pub fn load_versions() -> Result<GameVersions, String> {
    let games_dir = get_games_dir()?;
    let version_path = games_dir.join("version.json");

    if version_path.exists() {
        let content = fs::read_to_string(&version_path)
            .map_err(|e| format!("Failed to read version.json: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse version.json: {}", e))
    } else {
        // Return default versions
        Ok(GameVersions::default())
    }
}

/// Save version information to version.json
pub fn save_versions(versions: &GameVersions) -> Result<(), String> {
    let games_dir = get_games_dir()?;
    fs::create_dir_all(&games_dir)
        .map_err(|e| format!("Failed to create games directory: {}", e))?;

    let version_path = games_dir.join("version.json");
    let content = serde_json::to_string_pretty(versions)
        .map_err(|e| format!("Failed to serialize versions: {}", e))?;
    fs::write(&version_path, content).map_err(|e| format!("Failed to write version.json: {}", e))
}

/// Load user settings from settings.json
pub fn load_settings() -> Result<Settings, String> {
    let flash_dir = get_flash_dir()?;
    let settings_path = flash_dir.join("settings.json");

    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read settings.json: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings.json: {}", e))
    } else {
        Ok(Settings::default())
    }
}

/// Save user settings to settings.json
pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let flash_dir = get_flash_dir()?;
    fs::create_dir_all(&flash_dir)
        .map_err(|e| format!("Failed to create flash directory: {}", e))?;

    let settings_path = flash_dir.join("settings.json");
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    fs::write(&settings_path, content).map_err(|e| format!("Failed to write settings.json: {}", e))
}

/// Initialize the application directories and configuration
pub fn init_config() -> Result<(), String> {
    let games_dir = get_games_dir()?;
    let flash_dir = get_flash_dir()?;

    fs::create_dir_all(&games_dir)
        .map_err(|e| format!("Failed to create games directory: {}", e))?;
    fs::create_dir_all(&flash_dir)
        .map_err(|e| format!("Failed to create flash directory: {}", e))?;

    let ruffle_dir = get_ruffle_dir()?;
    fs::create_dir_all(&ruffle_dir)
        .map_err(|e| format!("Failed to create ruffle directory: {}", e))?;

    Ok(())
}

/// Get the flash player executable path based on OS and settings
pub fn get_flash_player_path(config: &AppConfig, settings: &Settings) -> Result<PathBuf, String> {
    // Check for custom path first
    if let Some(custom_path) = &settings.flash_player_path {
        let path = PathBuf::from(custom_path);
        if path.exists() {
            return Ok(path);
        }
    }

    // Use default path based on OS
    let flash_dir = get_flash_dir()?;

    #[cfg(target_os = "windows")]
    let filename = &config.flash_player.windows.filename;

    #[cfg(target_os = "macos")]
    let filename = &config.flash_player.macos.filename;

    #[cfg(target_os = "linux")]
    let filename = &config.flash_player.linux.filename;

    Ok(flash_dir.join(filename))
}

/// Get the ruffle executable path based on OS and settings
pub fn get_ruffle_path(config: &AppConfig, settings: &Settings) -> Result<PathBuf, String> {
    // Check for custom path first
    if let Some(custom_path) = &settings.ruffle_path {
        let path = PathBuf::from(custom_path);
        if path.exists() {
            return Ok(path);
        }
    }

    // Use default path based on OS
    let ruffle_dir = get_ruffle_dir()?;

    #[cfg(target_os = "windows")]
    let filename = &config.ruffle.windows.filename;

    #[cfg(target_os = "macos")]
    let filename = &config.ruffle.macos.filename;

    #[cfg(target_os = "linux")]
    let filename = &config.ruffle.linux.filename;

    Ok(ruffle_dir.join(filename))
}
