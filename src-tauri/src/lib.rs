mod config;
mod flash;
mod game;
mod ruffle;

use config::{AppConfig, Settings};
use std::path::PathBuf;
use std::sync::Mutex;

fn load_bundled_config() -> Result<AppConfig, String> {
    // In production, config.json is bundled as a Tauri resource.
    // During development, load from the src/assets directory.
    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("src")
        .join("assets")
        .join("config.json");

    if dev_path.exists() {
        return config::load_config(&dev_path);
    }

    // Hard-coded fallback config
    Ok(AppConfig::default())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        // Optional: Force Wayland if you want to avoid XWayland bugs
        // std::env::set_var("GDK_BACKEND", "wayland");
    }

    // Initialize config directories
    if let Err(e) = config::init_config() {
        eprintln!("Warning: Failed to initialize config directories: {}", e);
    }

    // Load configuration
    let app_config = match load_bundled_config() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load config: {}. Using default configuration.", e);
            AppConfig::default()
        }
    };

    // Load settings
    let settings = config::load_settings().unwrap_or_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(app_config)
        .manage(Mutex::new(settings))
        .invoke_handler(tauri::generate_handler![
            // Flash commands
            flash::check_flash_installed,
            flash::get_flash_path,
            flash::download_flash,
            // Ruffle commands
            ruffle::check_ruffle_installed,
            ruffle::get_ruffle_path,
            ruffle::download_ruffle,
            // Game commands
            game::is_game_downloaded,
            game::get_game_path,
            game::download_game,
            game::launch_game,
            // Settings commands
            get_settings,
            save_settings,
            // Install date commands
            get_flash_install_date,
            get_ruffle_install_date,
            // Update commands
            check_for_updates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_settings(settings: tauri::State<'_, Mutex<Settings>>) -> Settings {
    match settings.lock() {
        Ok(s) => s.clone(),
        Err(poisoned) => {
            // Recover inner value if mutex was poisoned
            poisoned.into_inner().clone()
        }
    }
}

#[tauri::command]
fn save_settings(
    new_settings: Settings,
    settings: tauri::State<'_, Mutex<Settings>>,
) -> Result<(), String> {
    match settings.lock() {
        Ok(mut s) => {
            *s = new_settings.clone();
        }
        Err(poisoned) => {
            let mut guard = poisoned.into_inner();
            *guard = new_settings.clone();
        }
    }

    config::save_settings(&new_settings)
}

/// Flash Player'ın kurulum tarihini döndürür (ISO 8601).
/// Hiç kurulmamışsa boş string döner.
#[tauri::command]
fn get_flash_install_date() -> String {
    config::load_versions()
        .map(|v| v.flash_installed_at)
        .unwrap_or_default()
}

/// Ruffle'ın kurulum tarihini döndürür (ISO 8601).
/// Hiç kurulmamışsa boş string döner.
#[tauri::command]
fn get_ruffle_install_date() -> String {
    config::load_versions()
        .map(|v| v.ruffle_installed_at)
        .unwrap_or_default()
}

/// Final URL'den `-v` sonrası versiyon string'ini çıkarır.
/// Örnek: `"https://ptd.onl/game/PTD1-v3.6.6.swf"` → `"3.6.6"`
fn parse_version_from_url(url: &str) -> Option<String> {
    let filename = url.split('/').last()?;
    if filename.contains("-v") {
        let after_v = filename.split("-v").nth(1)?;
        let version = after_v.split('.').collect::<Vec<_>>();
        // ".swf" uzantısını at: ["3","6","6","swf"] → "3.6.6"
        let version_parts: Vec<&str> = version
            .iter()
            .take_while(|p| p.chars().all(|c| c.is_ascii_digit()))
            .copied()
            .collect();
        if !version_parts.is_empty() {
            return Some(version_parts.join("."));
        }
    }
    None
}

/// HEAD isteği atıp redirect sonrası final URL'den versiyon çeker.
///
/// Öncelik sırası:
///   1. Final URL → dosya adı → `-v<versiyon>`
///   2. Last-Modified → Unix timestamp string (fallback)
///   3. None
async fn fetch_remote_version(
    client: &reqwest::Client,
    url: &str,
) -> Option<String> {
    let resp = client.head(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    // 1. Redirect sonrası final URL (örn. "https://ptd.onl/game/PTD1-v3.6.6.swf")
    if let Some(version) = parse_version_from_url(resp.url().as_str()) {
        return Some(version);
    }

    // 2. Last-Modified → timestamp string
    resp.headers()
        .get(reqwest::header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| chrono::DateTime::parse_from_rfc2822(s).ok())
        .map(|dt| dt.timestamp().to_string())
}

/// Checks each game URL with a HEAD request.
///
/// Reads the `Content-Disposition` header to extract a version string from the
/// remote filename (e.g. `ptd1-v1.2.3.swf` → `"1.2.3"`).  Falls back to the
/// `Last-Modified` timestamp when no versioned filename is present.
///
/// Returns the list of game IDs whose remote version differs from the locally
/// stored one, or that have never been downloaded.
#[tauri::command]
async fn check_for_updates(
    config: tauri::State<'_, AppConfig>,
) -> Result<Vec<String>, String> {
    let versions = config::load_versions().unwrap_or_default();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut updates: Vec<String> = Vec::new();

    // Iterate in a fixed, deterministic order so the returned list is always
    // PTD1 → PTD1_Hacked → PTD2 → PTD2_Hacked → PTD3 → PTD3_Hacked.
    let ordered_ids = ["PTD1", "PTD1_Hacked", "PTD2", "PTD2_Hacked", "PTD3", "PTD3_Hacked"];

    for game_id in &ordered_ids {
        let game_id = game_id.to_string();
        let url = match config.game_urls.get(&game_id) {
            Some(u) => u.clone(),
            None => continue, // not in config, skip
        };
        // If the game was never downloaded, it counts as "needs update"
        let local_version = match versions.games.get(&game_id) {
            Some(v) if !v.is_empty() => v.clone(),
            _ => {
                updates.push(game_id.clone());
                continue;
            }
        };

        // HEAD isteği → final URL'den versiyon
        match fetch_remote_version(&client, &url).await {
            Some(remote_version) if remote_version != local_version => {
                // Sayısal timestamp ise büyük/küçük karşılaştır,
                // semver string ise eşitsizlik güncelleme var demek.
                let needs_update = match (
                    local_version.parse::<i64>(),
                    remote_version.parse::<i64>(),
                ) {
                    (Ok(local_ts), Ok(remote_ts)) => remote_ts > local_ts,
                    _ => true, // semver farklıysa güncelleme var
                };
                if needs_update {
                    updates.push(game_id.clone());
                }
            }
            _ => {}
        }
    }

    Ok(updates)
}
