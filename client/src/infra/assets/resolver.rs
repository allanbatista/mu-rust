use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};

const DATA_PREFIX: &str = "data/";
const REMASTER_PREFIX: &str = "remaster/";

static ASSET_ROOT: OnceLock<RwLock<PathBuf>> = OnceLock::new();
static USE_REMASTER_ASSETS: AtomicBool = AtomicBool::new(true);

pub fn default_asset_root_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets"))
}

pub fn current_asset_root_path() -> PathBuf {
    match asset_root_lock().read() {
        Ok(path) => path.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

pub fn configure_asset_resolver(asset_root: impl Into<PathBuf>, use_remaster: bool) {
    let new_root = asset_root.into();
    match asset_root_lock().write() {
        Ok(mut root) => *root = new_root,
        Err(poisoned) => *poisoned.into_inner() = new_root,
    }
    set_use_remaster_assets(use_remaster);
}

pub fn use_remaster_assets_enabled() -> bool {
    USE_REMASTER_ASSETS.load(Ordering::Relaxed)
}

pub fn set_use_remaster_assets(enabled: bool) {
    USE_REMASTER_ASSETS.store(enabled, Ordering::Relaxed);
}

pub fn resolve_asset_path(path: &str) -> String {
    resolve_asset_path_with_mode(path, use_remaster_assets_enabled())
}

pub fn resolve_asset_path_with_mode(path: &str, use_remaster: bool) -> String {
    let normalized = normalize_asset_path(path);
    if normalized.is_empty() || !use_remaster {
        return normalized;
    }

    let (base_path, label_suffix) = split_asset_label(&normalized);
    let Some(remaster_candidate) = remaster_candidate_for(base_path) else {
        return normalized;
    };

    if path_exists_under_root(&remaster_candidate) {
        format!("{remaster_candidate}{label_suffix}")
    } else {
        normalized
    }
}

pub fn remaster_variant_exists(path: &str) -> bool {
    let normalized = normalize_asset_path(path);
    if normalized.is_empty() {
        return false;
    }

    let (base_path, _) = split_asset_label(&normalized);
    match remaster_candidate_for(base_path) {
        Some(remaster_candidate) => path_exists_under_root(&remaster_candidate),
        None => base_path.starts_with(REMASTER_PREFIX) && path_exists_under_root(base_path),
    }
}

pub fn asset_path_exists(path: &str) -> bool {
    let resolved = resolve_asset_path(path);
    asset_path_exists_exact(&resolved)
}

pub fn asset_path_exists_exact(path: &str) -> bool {
    let normalized = normalize_asset_path(path);
    if normalized.is_empty() {
        return false;
    }
    let (base_path, _) = split_asset_label(&normalized);
    path_exists_under_root(base_path)
}

fn asset_root_lock() -> &'static RwLock<PathBuf> {
    ASSET_ROOT.get_or_init(|| RwLock::new(default_asset_root_path()))
}

fn normalize_asset_path(raw_path: &str) -> String {
    raw_path
        .trim()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string()
}

fn split_asset_label(path: &str) -> (&str, &str) {
    match path.find('#') {
        Some(index) => (&path[..index], &path[index..]),
        None => (path, ""),
    }
}

fn remaster_candidate_for(base_path: &str) -> Option<String> {
    if !base_path.starts_with(DATA_PREFIX) || base_path.starts_with(REMASTER_PREFIX) {
        return None;
    }
    Some(format!("{REMASTER_PREFIX}{base_path}"))
}

fn path_exists_under_root(relative_path: &str) -> bool {
    let root = current_asset_root_path();
    root.join(relative_path).is_file()
}
