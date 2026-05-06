use std::path::Path;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Runtime};
use tauri_plugin_shell::{process::CommandChild, ShellExt};

/// Pick a free localhost port. We bind a TcpListener, read its port, and drop the
/// listener — there's a tiny race window where another process could grab the port,
/// but it's acceptable for a single-user desktop app.
fn pick_free_port() -> std::io::Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

pub struct SidecarHandle {
    pub child: CommandChild,
    pub port: u16,
}

pub fn spawn_sidecar<R: Runtime>(
    app: &AppHandle<R>,
    settings_path: &Path,
) -> Result<SidecarHandle, String> {
    let port = pick_free_port().map_err(|e| format!("failed to pick port: {e}"))?;
    let port_arg = port.to_string();
    let settings_arg = settings_path.to_string_lossy().into_owned();

    let (_rx, child) = app
        .shell()
        .sidecar("eduport-sidecar")
        .map_err(|e| format!("failed to locate bundled sidecar: {e}"))?
        .args([
            "--port",
            port_arg.as_str(),
            "--settings",
            settings_arg.as_str(),
        ])
        .spawn()
        .map_err(|e| format!("failed to spawn bundled sidecar: {e}"))?;

    Ok(SidecarHandle { child, port })
}

/// Poll GET /health until 200 OK or timeout.
pub fn wait_for_health(port: u16, timeout: Duration) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{port}/health");
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match reqwest::blocking::get(&url) {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    Err(format!(
        "sidecar did not respond on /health within {:?}",
        timeout
    ))
}

pub fn kill_sidecar(handle: SidecarHandle) {
    let _ = handle.child.kill();
}
