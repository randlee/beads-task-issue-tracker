use std::env;
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

// Global child process handle for beads-probe
pub(crate) static PROBE_CHILD: LazyLock<Mutex<Option<std::process::Child>>> =
    LazyLock::new(|| Mutex::new(None));

// ============================================================================
// External Data Source Commands
// ============================================================================

#[tauri::command]
pub(crate) async fn fetch_external_data(url: String) -> Result<String, String> {
    log_info!("[probe] GET {}", url);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let err = format!("HTTP {}: {}", response.status().as_u16(), response.status().canonical_reason().unwrap_or("Unknown"));
        log_error!("[probe] GET failed: {}", err);
        return Err(err);
    }

    response.text().await.map_err(|e| format!("Failed to read response: {}", e))
}

#[tauri::command]
pub(crate) async fn check_external_health(url: String) -> Result<bool, String> {
    let health_url = format!("{}/health", url.trim_end_matches('/'));
    log_info!("[probe] Health check: {}", health_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    match client.get(&health_url).send().await {
        Ok(response) => Ok(response.status().is_success()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub(crate) async fn post_external_data(url: String, body: String) -> Result<String, String> {
    log_info!("[probe] POST {}", url);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status.as_u16(), text));
    }

    response.text().await.map_err(|e| format!("Failed to read response: {}", e))
}

#[tauri::command]
pub(crate) async fn delete_external_data(url: String) -> Result<String, String> {
    log_info!("[probe] DELETE {}", url);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client
        .delete(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status.as_u16(), text));
    }

    response.text().await.map_err(|e| format!("Failed to read response: {}", e))
}

#[tauri::command]
pub(crate) async fn patch_external_data(url: String, body: String) -> Result<String, String> {
    log_info!("[probe] PATCH {}", url);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client
        .patch(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status.as_u16(), text));
    }

    response.text().await.map_err(|e| format!("Failed to read response: {}", e))
}

// ============================================================================
// Probe Launcher
// ============================================================================

#[tauri::command]
pub(crate) async fn launch_probe(port: u16) -> Result<String, String> {
    use std::process::Stdio;

    let health_url = format!("http://127.0.0.1:{}/health", port);

    // Check if probe is already reachable via HTTP health endpoint
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    if let Ok(resp) = client.get(&health_url).send().await {
        if resp.status().is_success() {
            log_info!("[probe] Already running on port {}", port);
            return Ok("already running".to_string());
        }
    }

    // Determine binary: BEADS_PROBE_BIN env var, fallback to "beads-probe"
    let bin = env::var("BEADS_PROBE_BIN").unwrap_or_else(|_| "beads-probe".to_string());
    log_info!("[probe] Launching: {} --port {}", bin, port);

    let child = Command::new(&bin)
        .arg("--port")
        .arg(port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {}", bin, e))?;

    // Store child handle so it lives as long as the app
    if let Ok(mut guard) = PROBE_CHILD.lock() {
        *guard = Some(child);
    }

    log_info!("[probe] Launched on port {}", port);
    Ok("launched".to_string())
}


