use crate::cli::reset_bd_version_cache;
use btit_cli::{command::new_command, path::get_extended_path};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

// Configurable CLI binary name (default: "bd")
pub(crate) static CLI_BINARY: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new("bd".to_string()));

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AppConfig {
    #[serde(default = "default_cli_binary")]
    pub(crate) cli_binary: String,
}

/// Serde default for `AppConfig::cli_binary`: auto-detects the CLI (`btit_cli::probe::default_cli_binary`).
fn default_cli_binary() -> String {
    btit_cli::probe::default_cli_binary()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            cli_binary: default_cli_binary(),
        }
    }
}

pub(crate) fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.beads.manager")
        .join("settings.json")
}

pub(crate) fn load_config() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => return config,
                Err(e) => log::warn!("[config] Failed to parse settings.json: {}", e),
            },
            Err(e) => log::warn!("[config] Failed to read settings.json: {}", e),
        }
    }
    AppConfig::default()
}

pub(crate) fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(&path, json)
        .map_err(|e| format!("Failed to write config: {}", e))?;
    Ok(())
}

pub(crate) fn get_cli_binary() -> String {
    CLI_BINARY.lock().unwrap().clone()
}

#[tauri::command]
pub(crate) async fn get_bd_version() -> String {
    let binary = get_cli_binary();
    match new_command(&binary)
        .arg("--version")
        .current_dir(std::env::temp_dir())
        .env("PATH", get_extended_path())
        .output()
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if binary != "bd" {
                format!("{} ({})", version, binary)
            } else {
                version
            }
        }
        _ => format!("{} not found", binary),
    }
}

// ============================================================================
// CLI Binary Configuration Commands
// ============================================================================

#[tauri::command]
pub(crate) async fn get_cli_binary_path() -> String {
    get_cli_binary()
}

#[tauri::command]
pub(crate) async fn set_cli_binary_path(path: String) -> Result<String, String> {
    let binary = if path.trim().is_empty() { "bd".to_string() } else { path.trim().to_string() };

    // Validate the binary first
    let version = validate_cli_binary_internal(&binary)?;

    // Update global state and reset version cache (new binary may be different version)
    *CLI_BINARY.lock().unwrap() = binary.clone();
    reset_bd_version_cache();

    // Persist to config file
    let mut config = load_config();
    config.cli_binary = binary.clone();
    save_config(&config)?;

    log_info!("[config] CLI binary set to: {} ({})", binary, version);
    Ok(version)
}

#[tauri::command]
pub(crate) async fn validate_cli_binary(path: String) -> Result<String, String> {
    let binary = if path.trim().is_empty() { "bd".to_string() } else { path.trim().to_string() };
    validate_cli_binary_internal(&binary)
}

pub(crate) fn validate_cli_binary_internal(binary: &str) -> Result<String, String> {
    // Security: reject shell metacharacters — Command::new() doesn't use a shell,
    // but defense-in-depth prevents any future misuse
    let forbidden = [';', '|', '&', '$', '`', '>', '<', '(', ')', '{', '}', '!', '\n', '\r'];
    if binary.chars().any(|c| forbidden.contains(&c)) {
        return Err("Invalid binary path: contains shell metacharacters".to_string());
    }
    if binary.contains("..") {
        return Err("Invalid binary path: directory traversal not allowed".to_string());
    }

    match new_command(binary)
        .arg("--version")
        .current_dir(std::env::temp_dir())
        .env("PATH", get_extended_path())
        .output()
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if version.is_empty() {
                Err(format!("'{}' returned empty version output", binary))
            } else {
                Ok(version)
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!("'{}' failed: {}", binary, if stderr.is_empty() { "unknown error".to_string() } else { stderr }))
        }
        Err(e) => {
            Err(format!("'{}' not found or not executable: {}", binary, e))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // ---- Config path (#8) -------------------------------------------------------

    #[test]
    fn get_config_path_ends_with_correct_suffix() {
        let path = get_config_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("com.beads.manager"));
        assert!(path_str.ends_with("settings.json"));
    }

    #[test]
    fn get_config_path_uses_path_components() {
        let path = get_config_path();
        let components: Vec<_> = path.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
        assert!(components.len() >= 2);
        assert_eq!(components[components.len() - 1], "settings.json");
        assert_eq!(components[components.len() - 2], "com.beads.manager");
    }


}
