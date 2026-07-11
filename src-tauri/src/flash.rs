use crate::config::{self, AppConfig, Settings};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
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
pub fn check_flash_installed(
    config: tauri::State<'_, AppConfig>,
    settings: tauri::State<'_, Mutex<Settings>>,
) -> bool {
    let settings = match settings.lock() {
        Ok(s) => s,
        Err(p) => p.into_inner(),
    };

    match config::get_flash_player_path(&config, &settings) {
        Ok(path) => path.exists(),
        Err(_) => false,
    }
}

#[tauri::command]
pub fn get_flash_path(
    config: tauri::State<'_, AppConfig>,
    settings: tauri::State<'_, Mutex<Settings>>,
) -> Result<String, String> {
    let settings = match settings.lock() {
        Ok(s) => s,
        Err(p) => p.into_inner(),
    };

    let path = config::get_flash_player_path(&config, &settings)?;
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid path".to_string())
}

#[tauri::command]
pub async fn download_flash(
    window: Window,
    config: tauri::State<'_, AppConfig>,
) -> Result<String, String> {
    // Get download info based on OS
    let flash_dir = config::get_flash_dir()?;
    fs::create_dir_all(&flash_dir)
        .map_err(|e| format!("Failed to create flash directory: {}", e))?;

    #[cfg(target_os = "windows")]
    let (primary_url, fallback_url, filename) = (
        &config.flash_player.windows.primary_url,
        &config.flash_player.windows.fallback_url,
        &config.flash_player.windows.filename,
    );

    #[cfg(target_os = "macos")]
    let (primary_url, fallback_url, filename) = (
        &config.flash_player.macos.primary_url,
        &config.flash_player.macos.fallback_url,
        "flash_player.dmg",
    );

    #[cfg(target_os = "linux")]
    let (primary_url, fallback_url, filename) = (
        &config.flash_player.linux.primary_url,
        &config.flash_player.linux.fallback_url,
        "flash_player.tar.gz",
    );

    let download_path = flash_dir.join(filename);

    // Emit initial progress
    let _ = window.emit(
        "download-progress",
        DownloadProgress {
            item: "flash_player".to_string(),
            progress: 0,
            downloaded: 0,
            total: 0,
            status: "Starting download...".to_string(),
        },
    );

    // Try primary URL first, then fallback if necessary
    let primary_attempt =
        download_file_with_progress(&window, primary_url, &download_path, "flash_player").await;
    if primary_attempt.is_err() {
        if let Some(fallback) = fallback_url {
            let _ = window.emit(
                "download-progress",
                DownloadProgress {
                    item: "flash_player".to_string(),
                    progress: 0,
                    downloaded: 0,
                    total: 0,
                    status: "Primary failed, trying fallback...".to_string(),
                },
            );
            download_file_with_progress(&window, fallback, &download_path, "flash_player").await?;
        } else {
            return Err(primary_attempt
                .err()
                .unwrap_or_else(|| "Download failed".to_string()));
        }
    }

    // Extract based on OS
    #[cfg(target_os = "linux")]
    {
        extract_tar_gz(&download_path, &flash_dir)?;
        let _ = fs::remove_file(&download_path);

        // Make executable
        let flash_bin = flash_dir.join(&config.flash_player.linux.filename);
        if flash_bin.exists() {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&flash_bin)
                .map_err(|e| format!("Failed to get permissions: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&flash_bin, perms)
                .map_err(|e| format!("Failed to set permissions: {}", e))?;
        }
    }

    #[cfg(target_os = "macos")]
    {
        extract_dmg(
            &download_path,
            &flash_dir,
            &config.flash_player.macos.filename,
        )?;
        let _ = fs::remove_file(&download_path);
    }

    // Update version info
    let mut versions = config::load_versions().unwrap_or_default();
    versions.flash_player = config.flash_player.fallback_version.clone();
    versions.flash_installed_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    config::save_versions(&versions)?;

    // Emit completion
    let _ = window.emit(
        "download-progress",
        DownloadProgress {
            item: "flash_player".to_string(),
            progress: 100,
            downloaded: 0,
            total: 0,
            status: "Download complete".to_string(),
        },
    );

    #[cfg(target_os = "windows")]
    let final_path = flash_dir.join(&config.flash_player.windows.filename);
    #[cfg(target_os = "macos")]
    let final_path = flash_dir.join(&config.flash_player.macos.filename);
    #[cfg(target_os = "linux")]
    let final_path = flash_dir.join(&config.flash_player.linux.filename);

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
    // Limit downloads to a reasonable maximum to avoid disk exhaustion
    const MAX_DOWNLOAD_SIZE: u64 = 500 * 1024 * 1024; // 500 MB

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let total = response.content_length().unwrap_or(0);
    if total > MAX_DOWNLOAD_SIZE {
        return Err(format!("Remote file too large: {} bytes", total));
    }

    let mut downloaded: u64 = 0;

    // Write to a temporary file first, then atomically rename into place
    let tmp_path = dest.with_extension("part");
    let mut file =
        fs::File::create(&tmp_path).map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        downloaded += chunk.len() as u64;

        if downloaded > MAX_DOWNLOAD_SIZE {
            let _ = fs::remove_file(&tmp_path);
            return Err("Download exceeded maximum allowed size".to_string());
        }

        file.write_all(&chunk)
            .map_err(|e| format!("Write error: {}", e))?;

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

    // Flush and rename
    file.flush()
        .map_err(|e| format!("Failed to flush file: {}", e))?;
    fs::rename(&tmp_path, dest).map_err(|e| format!("Failed to rename temp file: {}", e))?;

    Ok(())
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "macos")]
fn extract_dmg(dmg_path: &PathBuf, dest: &PathBuf, app_name: &str) -> Result<(), String> {
    use std::process::Command;

    let mount_point = std::env::temp_dir().join("ptd_flash_mount");
    fs::create_dir_all(&mount_point).map_err(|e| format!("Failed to create mount point: {}", e))?;

    // Mount DMG
    let out = Command::new("hdiutil")
        .args([
            "attach",
            dmg_path.to_str().ok_or("Invalid dmg path")?,
            "-mountpoint",
            mount_point.to_str().ok_or("Invalid mount point")?,
        ])
        .output()
        .map_err(|e| format!("Failed to mount DMG: {}", e))?;

    if !out.status.success() {
        return Err(format!(
            "hdiutil attach failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    // Copy app
    let source = mount_point.join(app_name);
    if source.exists() {
        fs_extra::dir::copy(&source, &dest, &fs_extra::dir::CopyOptions::new())
            .map_err(|e| format!("Failed to copy app: {}", e))?;
    }

    // Unmount DMG
    let out_un = Command::new("hdiutil")
        .args(["detach", mount_point.to_str().ok_or("Invalid mount point")?])
        .output();

    if let Ok(out_un) = out_un {
        if !out_un.status.success() {
            eprintln!(
                "Warning: failed to unmount DMG: {}",
                String::from_utf8_lossy(&out_un.stderr)
            );
        }
    }

    let _ = fs::remove_dir_all(&mount_point);

    Ok(())
}
