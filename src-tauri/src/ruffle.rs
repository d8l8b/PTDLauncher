use crate::config::{self, AppConfig, Settings};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tauri::{Emitter, Window};

#[derive(Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub item: String,
    pub progress: u32,
    pub downloaded: u64,
    pub total: u64,
    pub status: String,
}

use std::sync::Mutex;

#[tauri::command]
pub fn check_ruffle_installed(
    config: tauri::State<'_, AppConfig>,
    settings: tauri::State<'_, Mutex<Settings>>,
) -> bool {
    let settings = settings.lock().unwrap();
    match config::get_ruffle_path(&config, &settings) {
        Ok(path) => path.exists(),
        Err(_) => false,
    }
}

#[tauri::command]
pub fn get_ruffle_path(
    config: tauri::State<'_, AppConfig>,
    settings: tauri::State<'_, Mutex<Settings>>,
) -> Result<String, String> {
    let settings = settings.lock().unwrap();
    let path = config::get_ruffle_path(&config, &settings)?;
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid path".to_string())
}

#[derive(Debug, serde::Deserialize)]
struct RuffleAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, serde::Deserialize)]
struct RuffleRelease {
    tag_name: String,
    assets: Vec<RuffleAsset>,
}

async fn fetch_latest_nightly() -> Result<(String, String, String), String> {
    let client = reqwest::Client::builder()
        .user_agent("PTDLauncher")
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let url = "https://api.github.com/repos/ruffle-rs/ruffle/releases";
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch releases: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API error: {}", response.status()));
    }

    let releases: Vec<RuffleRelease> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse releases: {}", e))?;

    // Find the latest nightly release (usually the first one, but let's be sure it has assets)
    let release = releases
        .first()
        .ok_or_else(|| "No releases found".to_string())?;

    // Determine target asset name based on OS
    #[cfg(target_os = "windows")]
    let target_pattern = "windows-x86_64.zip";
    #[cfg(target_os = "macos")]
    let target_pattern = "macos-universal.tar.gz";
    #[cfg(target_os = "linux")]
    let target_pattern = "linux-x86_64.tar.gz";

    let asset = release
        .assets
        .iter()
        .find(|a| a.name.contains(target_pattern) && !a.name.contains("extension"))
        .ok_or_else(|| format!("No asset found for target: {}", target_pattern))?;

    let filename = if cfg!(target_os = "windows") {
        "ruffle.exe".to_string()
    } else {
        "ruffle".to_string()
    };

    Ok((
        asset.browser_download_url.clone(),
        filename,
        release.tag_name.clone(),
    ))
}

#[tauri::command]
pub async fn download_ruffle(
    window: Window,
    config: tauri::State<'_, AppConfig>,
) -> Result<String, String> {
    // Get download info based on OS
    let ruffle_dir = config::get_ruffle_dir()?;
    fs::create_dir_all(&ruffle_dir)
        .map_err(|e| format!("Failed to create ruffle directory: {}", e))?;

    // Try to fetch latest nightly
    let _ = window.emit(
        "download-progress",
        DownloadProgress {
            item: "ruffle".to_string(),
            progress: 0,
            downloaded: 0,
            total: 0,
            status: "Fetching latest nightly...".to_string(),
        },
    );

    let (url, filename, version_tag) = match fetch_latest_nightly().await {
        Ok(info) => info,
        Err(e) => {
            // Fallback to config
            let _ = window.emit(
                "download-progress",
                DownloadProgress {
                    item: "ruffle".to_string(),
                    progress: 0,
                    downloaded: 0,
                    total: 0,
                    status: format!("Failed to fetch latest: {}. Using fallback...", e),
                },
            );

            #[cfg(target_os = "windows")]
            let (url, filename) = (&config.ruffle.windows.url, &config.ruffle.windows.filename);

            #[cfg(target_os = "macos")]
            let (url, filename) = (&config.ruffle.macos.url, &config.ruffle.macos.filename);

            #[cfg(target_os = "linux")]
            let (url, filename) = (&config.ruffle.linux.url, &config.ruffle.linux.filename);

            (url.clone(), filename.clone(), "fallback".to_string())
        }
    };

    // Determine archive name from URL
    let archive_name = url.split('/').next_back().unwrap_or("ruffle_archive");
    let download_path = ruffle_dir.join(archive_name);

    // Emit initial progress
    let _ = window.emit(
        "download-progress",
        DownloadProgress {
            item: "ruffle".to_string(),
            progress: 0,
            downloaded: 0,
            total: 0,
            status: "Starting download...".to_string(),
        },
    );

    // Download the file
    download_file_with_progress(&window, &url, &download_path, "ruffle").await?;

    // Extract based on extension
    if archive_name.ends_with(".zip") {
        extract_zip(&download_path, &ruffle_dir)?;
    } else if archive_name.ends_with(".tar.gz") {
        extract_tar_gz(&download_path, &ruffle_dir)?;
    } else {
        return Err(format!("Unsupported archive format: {}", archive_name));
    }

    let _ = fs::remove_file(&download_path);

    // Make executable on unix
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let ruffle_bin = ruffle_dir.join(&filename);
        if ruffle_bin.exists() {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&ruffle_bin)
                .map_err(|e| format!("Failed to get permissions: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&ruffle_bin, perms)
                .map_err(|e| format!("Failed to set permissions: {}", e))?;
        }
    }

    // Update version info
    let mut versions = config::load_versions().unwrap_or_default();
    versions.ruffle = version_tag;
    versions.ruffle_installed_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    config::save_versions(&versions)?;

    // Emit completion
    let _ = window.emit(
        "download-progress",
        DownloadProgress {
            item: "ruffle".to_string(),
            progress: 100,
            downloaded: 0,
            total: 0,
            status: "Download complete".to_string(),
        },
    );

    let final_path = ruffle_dir.join(filename);

    final_path
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid path".to_string())
}

async fn download_file_with_progress(
    window: &Window,
    url: &str,
    dest: &PathBuf,
    item_name: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

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

    Ok(())
}

fn extract_zip(archive: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    let file = fs::File::open(archive).map_err(|e| format!("Failed to open archive: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {}", e))?;

    archive
        .extract(dest)
        .map_err(|e| format!("Failed to extract archive: {}", e))?;
    Ok(())
}

fn extract_tar_gz(archive: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let file = fs::File::open(archive).map_err(|e| format!("Failed to open archive: {}", e))?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive
        .unpack(dest)
        .map_err(|e| format!("Failed to extract archive: {}", e))?;
    Ok(())
}
