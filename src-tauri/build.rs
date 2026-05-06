use std::env;
use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn ensure_debug_sidecar_placeholder() {
    if env::var("PROFILE").ok().as_deref() == Some("release") {
        return;
    }

    let Ok(target) = env::var("TARGET") else {
        return;
    };
    let extension = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    let path = PathBuf::from("binaries").join(format!("eduport-sidecar-{target}{extension}"));
    println!("cargo:rerun-if-changed={}", path.display());
    if path.exists() {
        return;
    }

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if target.contains("windows") {
        let _ = fs::write(
            &path,
            b"Placeholder sidecar. Run scripts/build_sidecar.py before packaging.\n",
        );
    } else {
        let script = r#"#!/usr/bin/env sh
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR/../../../sidecar"
exec uv run eduport-sidecar "$@"
"#;
        if fs::write(&path, script).is_ok() {
            #[cfg(unix)]
            {
                let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o755));
            }
        }
    }
}

fn main() {
    ensure_debug_sidecar_placeholder();
    tauri_build::build()
}
