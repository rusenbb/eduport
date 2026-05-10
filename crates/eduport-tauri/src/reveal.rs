use std::fs;
use std::process::Command;

/// Reveal `path` in the OS file manager (Finder/Explorer/Files), with the file
/// selected when the platform supports it.
#[tauri::command]
pub fn reveal_in_file_manager(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer.exe")
            .arg(format!("/select,{}", path))
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        // The freedesktop `org.freedesktop.FileManager1.ShowItems` D-Bus
        // method opens the parent folder *and* selects the target file.
        // It's implemented by every major Linux file manager (Nautilus,
        // Nemo, Caja, Thunar ≥ 1.8, Dolphin, PCManFM). Fall back to
        // `xdg-open` on the parent folder if D-Bus is unavailable or
        // the FM1 interface isn't registered.
        let abs = std::path::Path::new(&path)
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from(&path));
        let uri = format!("file://{}", abs.display());

        let dbus = Command::new("dbus-send")
            .args([
                "--session",
                "--print-reply",
                "--dest=org.freedesktop.FileManager1",
                "--type=method_call",
                "/org/freedesktop/FileManager1",
                "org.freedesktop.FileManager1.ShowItems",
                &format!("array:string:{uri}"),
                "string:",
            ])
            .status();

        let dbus_ok = matches!(&dbus, Ok(s) if s.success());
        if !dbus_ok {
            let parent = std::path::Path::new(&path)
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::path::PathBuf::from("/"));
            Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn copy_file(source_path: String, destination_path: String) -> Result<(), String> {
    fs::copy(source_path, destination_path)
        .map(|_| ())
        .map_err(|e| format!("failed to copy file: {e}"))
}

#[tauri::command]
pub fn read_file_bytes(path: String) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("failed to read file: {e}"))
}
