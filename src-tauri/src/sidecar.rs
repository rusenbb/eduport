use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Pick a free localhost port. We bind a TcpListener, read its port, and drop the
/// listener — there's a tiny race window where another process could grab the port,
/// but it's acceptable for a single-user desktop app.
fn pick_free_port() -> std::io::Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

pub struct SidecarHandle {
    pub child: Child,
    pub port: u16,
}

pub fn spawn_sidecar() -> Result<SidecarHandle, String> {
    let port = pick_free_port().map_err(|e| format!("failed to pick port: {e}"))?;

    let child = Command::new("eduport-sidecar")
        .arg("--port")
        .arg(port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!(
            "failed to spawn eduport-sidecar (is it on PATH?): {e}"
        ))?;

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

pub fn kill_sidecar(handle: &mut SidecarHandle) {
    let _ = handle.child.kill();
    let _ = handle.child.wait();
}
