use crate::cli::{detect_cli_client, new_command, parse_bd_version};
use crate::config::get_bd_version;
use btit_types::CliClient;
use std::process::Command;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;

// ============================================================================
// Update Checker Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct UpdateInfo {
    #[serde(rename = "currentVersion")]
    pub current_version: String,
    #[serde(rename = "latestVersion")]
    pub latest_version: String,
    #[serde(rename = "hasUpdate")]
    pub has_update: bool,
    #[serde(rename = "releaseUrl")]
    pub release_url: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: Option<String>,
    pub platform: String,
    #[serde(rename = "releaseNotes")]
    pub release_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
    #[serde(default)]
    body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BdCliUpdateInfo {
    #[serde(rename = "currentVersion")]
    pub current_version: String,
    #[serde(rename = "latestVersion")]
    pub latest_version: String,
    #[serde(rename = "hasUpdate")]
    pub has_update: bool,
    #[serde(rename = "releaseUrl")]
    pub release_url: String,
}

// ============================================================================
// Update Checker
// ============================================================================

pub(crate) const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const GITHUB_RELEASES_URL: &str = "https://api.github.com/repos/w3dev33/beads-task-issue-tracker/releases/latest";

/// Get a GitHub token from `gh auth token` (if gh CLI is installed and authenticated).
/// Raises the API rate limit from 60/hour (anonymous) to 5,000/hour (authenticated).
pub(crate) fn get_github_token() -> Option<String> {
    // Check GITHUB_TOKEN env var first
    if let Ok(token) = env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            return Some(token);
        }
    }
    // Fall back to gh CLI
    let output = new_command("gh")
        .args(&["auth", "token"])
        .output()
        .ok()?;
    if output.status.success() {
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }
    None
}

/// Build a reqwest client with GitHub auth if available.
pub(crate) fn github_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("beads-task-issue-tracker")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

/// Add GitHub auth header to a request if a token is available.
pub(crate) fn with_github_auth(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match get_github_token() {
        Some(token) => req.bearer_auth(token),
        None => req,
    }
}

pub(crate) fn get_platform_string() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

pub(crate) fn find_platform_asset(assets: &[GitHubAsset]) -> Option<&GitHubAsset> {
    let suffix = if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "_macOS-ARM64.dmg"
        } else {
            "_macOS-Intel.dmg"
        }
    } else if cfg!(target_os = "windows") {
        "_Windows.msi"
    } else {
        "_Linux-amd64.AppImage"
    };

    assets.iter().find(|a| a.name.ends_with(suffix))
}

pub(crate) fn compare_versions(current: &str, latest: &str) -> bool {
    // Remove 'v' prefix if present
    let current = current.trim_start_matches('v');
    let latest = latest.trim_start_matches('v');

    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    };

    let current_parts = parse_version(current);
    let latest_parts = parse_version(latest);

    for i in 0..3 {
        let c = current_parts.get(i).copied().unwrap_or(0);
        let l = latest_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if c > l {
            return false;
        }
    }
    false
}

#[tauri::command]
pub(crate) async fn check_for_updates() -> Result<UpdateInfo, String> {
    let client = github_client()?;

    let response = with_github_auth(client.get(GITHUB_RELEASES_URL))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch releases: {}", e))?;

    // Handle 404 (no published releases yet)
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(UpdateInfo {
            current_version: CURRENT_VERSION.to_string(),
            latest_version: CURRENT_VERSION.to_string(),
            has_update: false,
            release_url: "https://github.com/w3dev33/beads-task-issue-tracker/releases".to_string(),
            download_url: None,
            platform: get_platform_string().to_string(),
            release_notes: None,
        });
    }

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    let latest_version = release.tag_name.trim_start_matches('v').to_string();
    let has_update = compare_versions(CURRENT_VERSION, &latest_version);

    let download_url = find_platform_asset(&release.assets)
        .map(|a| a.browser_download_url.clone());

    // Fetch CHANGELOG.md via GitHub API (raw.githubusercontent CDN ignores query params for caching)
    let changelog = with_github_auth(
        client
            .get("https://api.github.com/repos/w3dev33/beads-task-issue-tracker/contents/CHANGELOG.md")
            .header("Accept", "application/vnd.github.raw+json")
    )
        .send()
        .await
        .ok()
        .and_then(|r| if r.status().is_success() { Some(r) } else { None });
    let changelog_text = match changelog {
        Some(r) => r.text().await.ok(),
        None => None,
    };

    Ok(UpdateInfo {
        current_version: CURRENT_VERSION.to_string(),
        latest_version,
        has_update,
        release_url: release.html_url,
        download_url,
        platform: get_platform_string().to_string(),
        release_notes: changelog_text.or(release.body),
    })
}

#[tauri::command]
pub(crate) async fn check_for_updates_demo() -> Result<UpdateInfo, String> {
    let client = github_client()?;

    let response = with_github_auth(client.get(GITHUB_RELEASES_URL))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch releases: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    let latest_version = release.tag_name.trim_start_matches('v').to_string();

    let download_url = find_platform_asset(&release.assets)
        .map(|a| a.browser_download_url.clone());

    // Fetch CHANGELOG.md via GitHub API (raw.githubusercontent CDN ignores query params for caching)
    let changelog = with_github_auth(
        client
            .get("https://api.github.com/repos/w3dev33/beads-task-issue-tracker/contents/CHANGELOG.md")
            .header("Accept", "application/vnd.github.raw+json")
    )
        .send()
        .await
        .ok()
        .and_then(|r| if r.status().is_success() { Some(r) } else { None });
    let changelog_text = match changelog {
        Some(r) => r.text().await.ok(),
        None => None,
    };

    // Demo mode: force has_update = true, fake current version as 0.0.0
    Ok(UpdateInfo {
        current_version: "0.0.0".to_string(),
        latest_version,
        has_update: true,
        release_url: release.html_url,
        download_url,
        platform: get_platform_string().to_string(),
        release_notes: changelog_text.or(release.body),
    })
}

#[tauri::command]
pub(crate) async fn check_bd_cli_update() -> Result<BdCliUpdateInfo, String> {
    // Get current bd version
    let version_str = get_bd_version().await;
    if version_str.contains("not found") {
        return Err("bd CLI not found".to_string());
    }

    // Parse semver from version string
    let current_tuple = parse_bd_version(&version_str)
        .ok_or_else(|| format!("Could not parse version from: {}", version_str))?;
    let current_version = format!("{}.{}.{}", current_tuple.0, current_tuple.1, current_tuple.2);

    // Determine the correct GitHub repo based on client type (bd vs br)
    let client_type = detect_cli_client(&version_str);
    let api_url = match client_type {
        CliClient::Br => "https://api.github.com/repos/Dicklesworthstone/beads_rust/releases/latest",
        _ => "https://api.github.com/repos/steveyegge/beads/releases/latest",
    };
    let releases_url = match client_type {
        CliClient::Br => "https://github.com/Dicklesworthstone/beads_rust/releases",
        _ => "https://github.com/steveyegge/beads/releases",
    };

    let client = github_client()?;

    let response = with_github_auth(client.get(api_url))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch releases: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    let latest_version = release.tag_name.trim_start_matches('v').to_string();
    let has_update = compare_versions(&current_version, &latest_version);

    Ok(BdCliUpdateInfo {
        current_version,
        latest_version,
        has_update,
        release_url: releases_url.to_string(),
    })
}

#[tauri::command]
pub(crate) async fn download_and_install_update(download_url: String) -> Result<String, String> {
    log::info!("[download_update] Starting download from: {}", download_url);

    // Extract filename from URL
    let filename = download_url
        .rsplit('/')
        .next()
        .unwrap_or("update-download")
        .to_string();
    log::info!("[download_update] Target filename: {}", filename);

    // Download the file
    let client = reqwest::Client::builder()
        .user_agent("beads-task-issue-tracker")
        .build()
        .map_err(|e| {
            log::error!("[download_update] Failed to create HTTP client: {}", e);
            format!("Failed to create HTTP client: {}", e)
        })?;

    log::info!("[download_update] Sending GET request...");
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| {
            log::error!("[download_update] HTTP request failed: {} (url: {})", e, download_url);
            format!("Failed to download update: {}", e)
        })?;

    let status = response.status();
    let final_url = response.url().to_string();
    log::info!("[download_update] Response status: {} (final URL: {})", status, final_url);

    if !status.is_success() {
        log::error!("[download_update] Download failed with status: {} (url: {})", status, final_url);
        return Err(format!("Download failed with status: {}", status));
    }

    log::info!("[download_update] Reading response bytes...");
    let bytes = response
        .bytes()
        .await
        .map_err(|e| {
            log::error!("[download_update] Failed to read response bytes: {}", e);
            format!("Failed to read download bytes: {}", e)
        })?;
    log::info!("[download_update] Downloaded {} bytes", bytes.len());

    // Save to ~/Downloads
    let download_dir = dirs::download_dir()
        .ok_or_else(|| {
            log::error!("[download_update] Could not find Downloads directory");
            "Could not find Downloads directory".to_string()
        })?;

    let dest_path = download_dir.join(&filename);
    log::info!("[download_update] Saving to: {}", dest_path.display());
    fs::write(&dest_path, &bytes)
        .map_err(|e| {
            log::error!("[download_update] Failed to save file to {}: {}", dest_path.display(), e);
            format!("Failed to save file: {}", e)
        })?;

    let dest_str = dest_path.to_string_lossy().to_string();
    log::info!("[download_update] Saved successfully: {} ({} bytes)", dest_str, bytes.len());

    // On macOS, mount the DMG
    #[cfg(target_os = "macos")]
    {
        if filename.ends_with(".dmg") {
            log::info!("[download_update] Mounting DMG: {}", dest_str);
            Command::new("open")
                .arg(&dest_path)
                .spawn()
                .map_err(|e| {
                    log::error!("[download_update] Failed to open DMG: {}", e);
                    format!("Failed to open DMG: {}", e)
                })?;
        }
    }

    Ok(dest_str)
}



#[cfg(test)]
mod tests {
    use super::*;

    // ---- Update checker helpers (#4) --------------------------------------------

    #[test]
    fn get_platform_string_returns_non_empty() {
        let platform = get_platform_string();
        assert!(!platform.is_empty());
        assert!((platform == "macos" || platform == "windows" || platform == "linux"),
            "platform must be one of macos, windows, linux, got: {}", platform);
    }

    #[test]
    fn find_platform_asset_matches_macos_arm64() {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            let assets = vec![GitHubAsset {
                name: "App_macOS-ARM64.dmg".to_string(),
                browser_download_url: "https://example.com/download".to_string(),
            }];
            let result = find_platform_asset(&assets);
            assert!(result.is_some());
            assert_eq!(result.unwrap().name, "App_macOS-ARM64.dmg");
        }
    }

    #[test]
    fn find_platform_asset_returns_none_for_no_match() {
        let assets = vec![GitHubAsset {
            name: "App_Unknown.dmg".to_string(),
            browser_download_url: "https://example.com/download".to_string(),
        }];
        let result = find_platform_asset(&assets);
        // On macOS it won't match because suffix doesn't match
        #[cfg(target_os = "macos")]
        assert!(result.is_none());
    }

    #[test]
    fn compare_versions_detects_new_version() {
        assert!(compare_versions("1.0.0", "1.0.1") == true);
        assert!(compare_versions("1.0.0", "1.1.0") == true);
        assert!(compare_versions("1.0.0", "2.0.0") == true);
    }

    #[test]
    fn compare_versions_detects_same_version() {
        assert!(compare_versions("1.0.0", "1.0.0") == false);
        assert!(compare_versions("1.2.3", "1.2.3") == false);
    }

    #[test]
    fn compare_versions_detects_older_version() {
        assert!(compare_versions("1.0.1", "1.0.0") == false);
        assert!(compare_versions("1.1.0", "1.0.0") == false);
        assert!(compare_versions("2.0.0", "1.0.0") == false);
    }

    #[test]
    fn compare_versions_handles_v_prefix() {
        assert!(compare_versions("v1.0.0", "v1.0.1") == true);
        assert!(compare_versions("1.0.0", "v1.0.1") == true);
        assert!(compare_versions("v1.0.0", "1.0.1") == true);
    }

    #[test]
    fn compare_versions_handles_missing_parts() {
        assert!(compare_versions("1.0", "1.0.1") == true);
        assert!(compare_versions("1", "1.0.1") == true);
        assert!(compare_versions("1.0.0", "1") == false);
    }

    #[test]
    fn compare_versions_with_prerelease_parses_numeric_parts_only() {
        // parse_version() uses filter_map(parse::<u32>) so "1.0.0-alpha" -> [1, 0, 0]
        // and "1.0.0-beta" -> [1, 0, 0], so they compare equal
        assert!(compare_versions("1.0.0-alpha", "1.0.0-beta") == false);
        // But "1.0.0-alpha" < "1.1.0-beta"
        assert!(compare_versions("1.0.0-alpha", "1.1.0-beta") == true);
    }


}
