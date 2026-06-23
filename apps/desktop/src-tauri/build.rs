use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

const STAGED_TOGOSPACE_RELATIVE: &str = "resources/extensions/TogoSpace";

fn main() {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is required during build"),
    );
    let staged_root = manifest_dir.join(STAGED_TOGOSPACE_RELATIVE);
    let should_stage = should_stage_togospace();

    println!("cargo:rerun-if-env-changed=TOGOSPACE_ROOT");
    println!("cargo:rerun-if-env-changed=PROFILE");

    if should_stage {
        let togospace_root = resolve_togospace_root(&manifest_dir)
            .expect("Unable to locate the TogoSpace repository root for desktop packaging");
        println!("cargo:rerun-if-changed={}", togospace_root.display());

        sync_repo_tree(&togospace_root, &staged_root)
            .expect("Failed to stage TogoSpace resources for desktop packaging");
    } else {
        fs::create_dir_all(&staged_root)
            .expect("Failed to prepare the empty TogoSpace resource directory for dev mode");
    }

    tauri_build::build()
}

fn should_stage_togospace() -> bool {
    env::var("PROFILE").map(|profile| profile == "release").unwrap_or(false)
}

fn resolve_togospace_root(manifest_dir: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(path) = env::var("TOGOSPACE_ROOT") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            candidates.push(PathBuf::from(trimmed));
        }
    }

    candidates.push(manifest_dir.join("../../../../"));
    candidates.push(manifest_dir.join("../../../external/TogoSpace"));

    candidates
        .into_iter()
        .find(|path| is_togospace_root(path))
        .and_then(|path| path.canonicalize().ok().or(Some(path)))
}

fn is_togospace_root(path: &Path) -> bool {
    path.join("scripts/start_backend.sh").exists() && path.join("src/backend_main.py").exists()
}

fn sync_repo_tree(source_root: &Path, destination_root: &Path) -> io::Result<()> {
    if destination_root.exists() {
        fs::remove_dir_all(destination_root)?;
    }

    fs::create_dir_all(destination_root)?;
    copy_dir_recursive(source_root, source_root, destination_root)
}

fn copy_dir_recursive(source_root: &Path, current_source: &Path, current_destination: &Path) -> io::Result<()> {
    for entry in fs::read_dir(current_source)? {
        let entry = entry?;
        let source_path = entry.path();
        let relative_path = source_path
            .strip_prefix(source_root)
            .expect("source tree traversal must remain under source_root");
        let file_type = entry.file_type()?;

        if should_skip_path(relative_path, file_type.is_dir()) {
            continue;
        }

        let destination_path = current_destination.join(entry.file_name());
        if file_type.is_dir() {
            fs::create_dir_all(&destination_path)?;
            copy_dir_recursive(source_root, &source_path, &destination_path)?;
            continue;
        }

        if file_type.is_file() {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &destination_path)?;
            copy_permissions(&source_path, &destination_path)?;
        }
    }

    Ok(())
}

fn should_skip_path(relative_path: &Path, is_dir: bool) -> bool {
    let normalized = relative_path.to_string_lossy().replace('\\', "/");
    if normalized.is_empty() {
        return false;
    }

    let file_name = relative_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let path_components: Vec<&str> = normalized.split('/').collect();

    if path_components.iter().any(|component| {
        matches!(
            *component,
            ".git"
                | ".idea"
                | ".mypy_cache"
                | ".pytest_cache"
                | ".venv"
                | "__pycache__"
                | "node_modules"
                | "target"
        )
    }) {
        return true;
    }

    if matches!(file_name, ".DS_Store" | ".coverage") {
        return true;
    }

    if file_name.ends_with(".pyc")
        || file_name.ends_with(".pyo")
        || file_name.ends_with(".log")
        || file_name.ends_with(".pid")
    {
        return true;
    }

    if matches!(
        normalized.as_str(),
        "build"
            | "dist"
            | "logs"
            | "run"
            | "data"
            | "test_data"
            | "frontend"
            | "workspace"
            | "workspace_root"
            | "dev_storage_root"
    ) {
        return true;
    }

    if normalized.starts_with("frontend/.dbg")
        || normalized.starts_with("frontend/apps/desktop/dist")
        || normalized.starts_with("frontend/apps/desktop/src-tauri/resources")
        || normalized.starts_with("frontend/apps/desktop/src-tauri/target")
        || normalized.starts_with("frontend/node_modules")
    {
        return true;
    }

    if is_dir && file_name == "logs" {
        return true;
    }

    false
}

fn copy_permissions(source_path: &Path, destination_path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = fs::metadata(source_path)?.permissions().mode();
        fs::set_permissions(destination_path, fs::Permissions::from_mode(mode))?;
    }

    #[cfg(not(unix))]
    {
        let _ = source_path;
        let _ = destination_path;
    }

    Ok(())
}
