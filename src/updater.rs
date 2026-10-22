use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;

const GITHUB_API_URL: &str = "https://api.github.com/repos/snakkarike/DisplayWarp/releases/latest";
const USER_AGENT: &str = "DisplayWarp-App";

#[derive(Clone, Debug, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate {
        checked_version: String,
    },
    UpdateAvailable {
        version: String,
        release_name: String,
        release_notes: String,
        download_url: String,
        html_url: String,
        asset_size: u64,
    },
    Downloading {
        version: String,
        downloaded_bytes: u64,
        total_bytes: u64,
        progress: f32, // 0.0 to 1.0
    },
    ReadyToRestart {
        version: String,
    },
    Error(String),
}

impl Default for UpdateStatus {
    fn default() -> Self {
        Self::Idle
    }
}

pub struct UpdateManager {
    pub status: Arc<Mutex<UpdateStatus>>,
    pub is_busy: Arc<AtomicBool>,
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateManager {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(UpdateStatus::Idle)),
            is_busy: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check GitHub for latest release in a background thread
    pub fn check_for_updates(&self) {
        if self.is_busy.swap(true, Ordering::SeqCst) {
            return; // Already in progress
        }

        let status_arc = Arc::clone(&self.status);
        let is_busy_arc = Arc::clone(&self.is_busy);

        *status_arc.lock() = UpdateStatus::Checking;

        std::thread::spawn(move || {
            let result = Self::perform_check();
            match result {
                Ok(status) => {
                    *status_arc.lock() = status;
                }
                Err(err) => {
                    *status_arc.lock() = UpdateStatus::Error(err);
                }
            }
            is_busy_arc.store(false, Ordering::SeqCst);
        });
    }

    fn perform_check() -> Result<UpdateStatus, String> {
        let current_version = env!("CARGO_PKG_VERSION");

        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(8))
            .timeout_read(Duration::from_secs(10))
            .user_agent(USER_AGENT)
            .build();

        let response = agent
            .get(GITHUB_API_URL)
            .set("Accept", "application/vnd.github.v3+json")
            .call()
            .map_err(|e| format!("Failed to connect to GitHub releases: {}", e))?;

        let body: serde_json::Value = response
            .into_json()
            .map_err(|e| format!("Invalid release response format: {}", e))?;

        let tag_name = body["tag_name"]
            .as_str()
            .ok_or_else(|| "Missing tag_name in release".to_string())?
            .trim();

        let release_version = tag_name.trim_start_matches('v');
        let release_name = body["name"].as_str().unwrap_or(tag_name).to_string();
        let release_notes = body["body"].as_str().unwrap_or("").to_string();
        let html_url = body["html_url"].as_str().unwrap_or("").to_string();

        if !is_newer_version(release_version, current_version) {
            return Ok(UpdateStatus::UpToDate {
                checked_version: current_version.to_string(),
            });
        }

        // Find DisplayWarp.exe asset
        let assets = body["assets"]
            .as_array()
            .ok_or_else(|| "No release assets found".to_string())?;

        let mut download_url = None;
        let mut asset_size = 0u64;

        for asset in assets {
            if let Some(name) = asset["name"].as_str() {
                if name.eq_ignore_ascii_case("DisplayWarp.exe") {
                    download_url = asset["browser_download_url"].as_str().map(|s| s.to_string());
                    asset_size = asset["size"].as_u64().unwrap_or(0);
                    break;
                }
            }
        }

        // Fallback: look for any .exe asset
        if download_url.is_none() {
            for asset in assets {
                if let Some(name) = asset["name"].as_str() {
                    if name.ends_with(".exe") {
                        download_url = asset["browser_download_url"].as_str().map(|s| s.to_string());
                        asset_size = asset["size"].as_u64().unwrap_or(0);
                        break;
                    }
                }
            }
        }

        let download_url = download_url.ok_or_else(|| {
            format!(
                "Release {} found, but DisplayWarp.exe was not in release assets.",
                tag_name
            )
        })?;

        Ok(UpdateStatus::UpdateAvailable {
            version: release_version.to_string(),
            release_name,
            release_notes,
            download_url,
            html_url,
            asset_size,
        })
    }

    /// Download the new binary in a background thread
    pub fn start_download(&self, version: String, download_url: String) {
        if self.is_busy.swap(true, Ordering::SeqCst) {
            return;
        }

        let status_arc = Arc::clone(&self.status);
        let is_busy_arc = Arc::clone(&self.is_busy);

        *status_arc.lock() = UpdateStatus::Downloading {
            version: version.clone(),
            downloaded_bytes: 0,
            total_bytes: 0,
            progress: 0.0,
        };

        std::thread::spawn(move || {
            let res = Self::perform_download(&version, &download_url, &status_arc);
            match res {
                Ok(_) => {
                    *status_arc.lock() = UpdateStatus::ReadyToRestart { version };
                }
                Err(e) => {
                    *status_arc.lock() = UpdateStatus::Error(e);
                }
            }
            is_busy_arc.store(false, Ordering::SeqCst);
        });
    }

    fn perform_download(
        version: &str,
        download_url: &str,
        status_arc: &Arc<Mutex<UpdateStatus>>,
    ) -> Result<(), String> {
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Could not get current executable path: {}", e))?;

        let new_exe = current_exe.with_extension("new.exe");

        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(15))
            .timeout_read(Duration::from_secs(60))
            .user_agent(USER_AGENT)
            .build();

        let response = agent
            .get(download_url)
            .call()
            .map_err(|e| format!("Failed to download update: {}", e))?;

        let total_bytes = response
            .header("Content-Length")
            .and_then(|h| h.parse::<u64>().ok())
            .unwrap_or(0);

        let mut reader = response.into_reader();
        let mut file = std::fs::File::create(&new_exe)
            .map_err(|e| format!("Failed to create temporary update file: {}", e))?;

        let mut buffer = [0u8; 16384];
        let mut downloaded_bytes = 0u64;

        loop {
            let bytes_read = reader
                .read(&mut buffer)
                .map_err(|e| format!("Error while downloading: {}", e))?;

            if bytes_read == 0 {
                break;
            }

            std::io::Write::write_all(&mut file, &buffer[..bytes_read])
                .map_err(|e| format!("Error writing update file: {}", e))?;

            downloaded_bytes += bytes_read as u64;

            let progress = if total_bytes > 0 {
                (downloaded_bytes as f32 / total_bytes as f32).min(1.0)
            } else {
                0.0
            };

            // Update status periodically
            if let Some(mut status) = status_arc.try_lock() {
                *status = UpdateStatus::Downloading {
                    version: version.to_string(),
                    downloaded_bytes,
                    total_bytes,
                    progress,
                };
            }
        }

        // Flush and verify non-empty file
        std::io::Write::flush(&mut file).map_err(|e| format!("Flush failed: {}", e))?;
        drop(file);

        let metadata = std::fs::metadata(&new_exe)
            .map_err(|e| format!("Failed to verify downloaded update: {}", e))?;

        if metadata.len() < 100_000 {
            let _ = std::fs::remove_file(&new_exe);
            return Err("Downloaded update file appears corrupted or incomplete.".to_string());
        }

        Ok(())
    }

    /// Swap the current running executable with the newly downloaded one and restart
    pub fn apply_and_restart() -> Result<(), String> {
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Cannot locate current executable: {}", e))?;

        let new_exe = current_exe.with_extension("new.exe");
        let old_exe = current_exe.with_extension("old.exe");

        if !new_exe.exists() {
            return Err("Downloaded update file not found.".to_string());
        }

        if old_exe.exists() {
            let _ = std::fs::remove_file(&old_exe);
        }

        // 1. Rename running exe -> .old.exe (allowed by Windows OS)
        std::fs::rename(&current_exe, &old_exe)
            .map_err(|e| format!("Failed to rename running executable: {}", e))?;

        // 2. Rename .new.exe -> current_exe
        if let Err(e) = std::fs::rename(&new_exe, &current_exe) {
            // Rollback on failure
            let _ = std::fs::rename(&old_exe, &current_exe);
            return Err(format!("Failed to install new executable: {}", e));
        }

        // 3. Launch newly installed executable
        std::process::Command::new(&current_exe)
            .spawn()
            .map_err(|e| format!("Failed to launch updated version: {}", e))?;

        // 4. Terminate this process
        std::process::exit(0);
    }
}

/// Clean up leftover .old.exe or .new.exe files on startup
pub fn cleanup_old_executables() {
    if let Ok(exe) = std::env::current_exe() {
        let old_exe = exe.with_extension("old.exe");
        if old_exe.exists() {
            let _ = std::fs::remove_file(old_exe);
        }
        let new_exe = exe.with_extension("new.exe");
        if new_exe.exists() {
            let _ = std::fs::remove_file(new_exe);
        }
    }
}

/// Parse semantic version string (e.g., "1.2.3")
pub fn parse_semver(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().trim_start_matches('v');
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().unwrap_or("0").split('-').next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// Returns true if remote version is strictly greater than local version
pub fn is_newer_version(remote: &str, local: &str) -> bool {
    match (parse_semver(remote), parse_semver(local)) {
        (Some(r), Some(l)) => r > l,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparisons() {
        assert!(is_newer_version("v1.2.0", "1.1.1"));
        assert!(is_newer_version("1.1.2", "1.1.1"));
        assert!(is_newer_version("2.0.0", "1.9.9"));
        assert!(!is_newer_version("1.1.1", "1.1.1"));
        assert!(!is_newer_version("v1.1.1", "1.1.1"));
        assert!(!is_newer_version("1.1.0", "1.1.1"));
        assert!(!is_newer_version("1.0.9", "1.1.0"));
    }
}
