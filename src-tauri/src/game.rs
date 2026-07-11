use crate::config::{self, AppConfig, Settings};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tauri::{Emitter, Window};

use crate::flash::DownloadProgress;

/// Extracts the version string after `-v` from the final URL or filename.
/// Example: `"https://ptd.onl/game/PTD1-v3.6.6.swf"` → `"3.6.6"`
fn parse_version_from_url(url: &str) -> Option<String> {
    let filename = url.split('/').last()?;
    if filename.contains("-v") {
        let after_v = filename.split("-v").nth(1)?;
        let version_parts: Vec<&str> = after_v
            .split('.')
            .take_while(|p| p.chars().all(|c| c.is_ascii_digit()))
            .collect();
        if !version_parts.is_empty() {
            return Some(version_parts.join("."));
        }
    }
    None
}

fn find_game_path(game_id: &str) -> Result<Option<PathBuf>, String> {
    let games_dir = config::get_games_dir()?;

    // Check for standard format first
    let standard_path = games_dir.join(format!("{}.swf", game_id));
    if standard_path.exists() {
        return Ok(Some(standard_path));
    }

    // Look for versioned files
    if let Ok(entries) = fs::read_dir(&games_dir) {
        let prefix = format!("{}-v", game_id);
        let mut latest_path: Option<PathBuf> = None;
        let mut latest_time = std::time::SystemTime::UNIX_EPOCH;

        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(&prefix) && name.ends_with(".swf") {
                    if let Ok(metadata) = path.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if modified > latest_time {
                                latest_time = modified;
                                latest_path = Some(path);
                            }
                        }
                    }
                }
            }
        }

        if latest_path.is_some() {
            return Ok(latest_path);
        }
    }

    Ok(None)
}

#[tauri::command]
pub fn is_game_downloaded(game_id: String) -> bool {
    find_game_path(&game_id).ok().flatten().is_some()
}

#[tauri::command]
pub fn get_game_path(game_id: String) -> Result<Option<String>, String> {
    find_game_path(&game_id).map(|opt| opt.and_then(|p| p.to_str().map(|s| s.to_string())))
}

#[tauri::command]
pub async fn download_game(
    window: Window,
    game_id: String,
    config: tauri::State<'_, AppConfig>,
) -> Result<String, String> {
    let url = config
        .game_urls
        .get(&game_id)
        .ok_or_else(|| format!("Game '{}' not found in configuration", game_id))?;

    let games_dir = config::get_games_dir()?;
    fs::create_dir_all(&games_dir)
        .map_err(|e| format!("Failed to create games directory: {}", e))?;

    let dest_path = games_dir.join(format!("{}.swf", game_id));

    // Emit initial progress
    let _ = window.emit(
        "download-progress",
        DownloadProgress {
            item: game_id.clone(),
            progress: 0,
            downloaded: 0,
            total: 0,
            status: "Starting download...".to_string(),
        },
    );

    // Download the file; the returned Option<String> is:
    //   Some("1.23")        → version from Content-Disposition
    //   Some("1741776693")  → Last-Modified timestamp
    //   None                → no relevant header in the GET response
    let remote_version = download_file_with_progress(&window, url, &dest_path, &game_id).await?;

    // If None, try fetching Last-Modified separately via a HEAD request;
    // if that also fails, fall back to the current Utc::now() timestamp as a last resort.
    let version_to_store = match remote_version {
        Some(v) => v,
        None => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default();
            client
                .head(url)
                .send()
                .await
                .ok()
                .filter(|r| r.status().is_success())
                .and_then(|r| {
                    r.headers()
                        .get(reqwest::header::LAST_MODIFIED)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| chrono::DateTime::parse_from_rfc2822(s).ok())
                        .map(|dt| dt.timestamp().to_string())
                })
                .unwrap_or_else(|| chrono::Utc::now().timestamp().to_string())
        }
    };

    let mut versions = config::load_versions().unwrap_or_default();
    versions.games.insert(game_id.clone(), version_to_store);
    config::save_versions(&versions)?;

    // Emit completion
    let _ = window.emit(
        "download-progress",
        DownloadProgress {
            item: game_id,
            progress: 100,
            downloaded: 0,
            total: 0,
            status: "Download complete".to_string(),
        },
    );

    dest_path
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid path".to_string())
}

use std::sync::Mutex;

#[tauri::command]
pub async fn launch_game(
    game_id: String,
    config: tauri::State<'_, AppConfig>,
    settings: tauri::State<'_, Mutex<Settings>>,
) -> Result<(), String> {
    let settings = match settings.lock() {
        Ok(s) => s,
        Err(p) => p.into_inner(),
    };

    // Find the game path
    let game_path = find_game_path(&game_id)?
        .ok_or_else(|| format!("Game '{}' not found. Please download it first.", game_id))?;

    // Determine which player to use
    let use_ruffle = settings.use_ruffle.unwrap_or(false);

    let player_path = if use_ruffle {
        let path = config::get_ruffle_path(&config, &settings)?;
        if !path.exists() {
            return Err("Ruffle not installed. Please download it first.".to_string());
        }
        path
    } else {
        let path = config::get_flash_player_path(&config, &settings)?;
        if !path.exists() {
            return Err("Flash Player not installed. Please download it first.".to_string());
        }
        path
    };

    // Get game URL for Ruffle arguments
    let game_url = config
        .game_urls
        .get(&game_id)
        .ok_or_else(|| format!("Game '{}' not found in configuration", game_id))?;

    // Derive base URL (remove filename from URL)
    let base_url = if let Some(idx) = game_url.rfind('/') {
        &game_url[..=idx]
    } else {
        game_url
    };

    // Launch the game
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new(&player_path);

        if use_ruffle {
            cmd.arg(&game_path)
                .arg("--spoof-url")
                .arg(game_url)
                .arg("--base")
                .arg(base_url);
        } else {
            cmd.arg(&game_path);
        }

        cmd.spawn()
            .map_err(|e| format!("Failed to launch game: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        if use_ruffle {
            // Ruffle is a binary, not an .app bundle usually
            Command::new(&player_path)
                .arg(&game_path)
                .arg("--spoof-url")
                .arg(game_url)
                .arg("--base")
                .arg(base_url)
                .spawn()
                .map_err(|e| format!("Failed to launch game: {}", e))?;
        } else {
            // Flash Player is an .app bundle
            let player_str = player_path
                .to_str()
                .ok_or_else(|| "Invalid player path".to_string())?;
            let out = Command::new("open")
                .args(["-a", player_str])
                .arg(&game_path)
                .spawn()
                .map_err(|e| format!("Failed to launch game: {}", e))?;
            let _ = out;
        }
    }

    #[cfg(target_os = "linux")]
    {
        let mut cmd = Command::new(&player_path);

        if use_ruffle {
            cmd.arg(&game_path)
                .arg("--spoof-url")
                .arg(game_url)
                .arg("--base")
                .arg(base_url);
        } else {
            cmd.arg(&game_path);
        }

        cmd.spawn()
            .map_err(|e| format!("Failed to launch game: {}", e))?;
    }

    Ok(())
}

async fn download_file_with_progress(
    window: &Window,
    url: &str,
    dest: &PathBuf,
    item_name: &str,
) -> Result<Option<String>, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    // Grab version from the final (redirect) URL before consuming the body.
    // e.g. "https://ptd.onl/game/PTD1-v3.6.6.swf" → "3.6.6"
    let url_version = parse_version_from_url(response.url().as_str());

    let last_modified_ts = response
        .headers()
        .get(reqwest::header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| chrono::DateTime::parse_from_rfc2822(s).ok())
        .map(|dt| dt.timestamp().to_string());

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file = fs::File::create(dest).map_err(|e| format!("Failed to create file: {}", e))?;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Write error: {}", e))?;

        downloaded += chunk.len() as u64;
        let progress = if total > 0 {
            ((downloaded as f64 / total as f64) * 100.0) as u32
        } else {
            0
        };

        let _ = window.emit(
            "download-progress",
            DownloadProgress {
                item: item_name.to_string(),
                progress,
                downloaded,
                total,
                status: "Downloading...".to_string(),
            },
        );
    }

    // Version priority order:
    //   1. Final URL → after "-v" (e.g. "PTD1-v3.6.6.swf" → "3.6.6")
    //   2. Last-Modified → timestamp string
    let version = url_version.or(last_modified_ts);

    Ok(version)
}
