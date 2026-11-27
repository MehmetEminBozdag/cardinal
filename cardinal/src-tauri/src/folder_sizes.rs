//! Folder size view: Tauri commands around [`folder_size::SizeEngine`].
//!
//! The engine keeps a per-directory size tree. FSEvents that Cardinal already
//! receives for its search index are forwarded here, so sizes stay current
//! without rescanning the disk. The tree is snapshotted next to `cardinal.db`
//! and shown immediately on the next launch, followed by a fresh scan.

use crate::size_activity::SizeActivity;
use cardinal_sdk::{EventFlag, FsEvent, ScanType};
use folder_size::{Listing, SizeEngine, Status, Suggestion, VolumeSpace, volume_space};
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::{Condvar, Mutex};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::AppHandle;
use tracing::{info, warn};

static ACTIVITY: Lazy<Mutex<SizeActivity>> = Lazy::new(|| Mutex::new(SizeActivity::default()));
static ACTIVITY_CHANGED: Condvar = Condvar::new();

pub fn set_foreground(foreground: bool) {
    let mut activity = ACTIVITY.lock();
    activity.set_foreground(foreground, Instant::now());
    ENGINE.set_active(activity.allowed(Instant::now()));
    ACTIVITY_CHANGED.notify_all();
}

#[tauri::command]
pub fn set_size_view_active(app: AppHandle, active: bool) {
    let foreground = crate::window_controls::is_main_window_foreground(&app);
    let mut activity = ACTIVITY.lock();
    activity.selected = active;
    activity.set_foreground(foreground, Instant::now());
    ENGINE.set_active(activity.allowed(Instant::now()));
    ACTIVITY_CHANGED.notify_all();
}

static ENGINE: Lazy<Arc<SizeEngine>> = Lazy::new(|| {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| "/".into());
    Arc::new(SizeEngine::new(home))
});
static SNAPSHOT_PATH: OnceCell<PathBuf> = OnceCell::new();

const PROCESS_INTERVAL: Duration = Duration::from_millis(1_500);
const SAVE_INTERVAL: Duration = Duration::from_secs(60);
const SNAPSHOT_REFRESH_AGE: Duration = Duration::from_secs(10 * 60);

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

/// Loads the previous snapshot (if any), refreshes it in the background and
/// starts the worker that applies pending file-system changes.
pub fn start(snapshot_path: PathBuf) {
    ENGINE.set_active(false);
    std::thread::Builder::new()
        .name("size-activity-gate".into())
        .spawn(|| {
            let mut activity = ACTIVITY.lock();
            loop {
                let now = Instant::now();
                ENGINE.set_active(activity.allowed(now));
                if let Some(remaining) = activity.remaining(now) {
                    ACTIVITY_CHANGED.wait_for(&mut activity, remaining);
                } else {
                    ACTIVITY_CHANGED.wait(&mut activity);
                }
            }
        })
        .expect("failed to start size activity gate");
    let _ = SNAPSHOT_PATH.set(snapshot_path.clone());
    let engine = Arc::clone(&ENGINE);
    std::thread::Builder::new()
        .name("folder-size-worker".into())
        .spawn(move || {
            match engine.load(&snapshot_path) {
                Ok(true) => {}
                Ok(false) => {}
                Err(error) if snapshot_path.exists() => {
                    warn!(?error, "could not read folder size snapshot");
                }
                Err(_) => {}
            }
            let mut last_save = Instant::now();
            loop {
                std::thread::sleep(PROCESS_INTERVAL);
                if !ACTIVITY.lock().allowed(Instant::now()) {
                    continue;
                }
                let status = engine.status();
                if status.phase == folder_size::Phase::Idle
                    && let Some(home) = std::env::var_os("HOME")
                {
                    engine.start_scan(PathBuf::from(home));
                }
                if status.from_snapshot
                    && now_secs().saturating_sub(status.scanned_at.unwrap_or(0))
                        >= SNAPSHOT_REFRESH_AGE.as_secs()
                    && let Some(root) = engine.root_path()
                {
                    info!(root = %root.display(), "refreshing visible folder size snapshot");
                    engine.start_scan(root);
                }
                engine.process_pending();
                if last_save.elapsed() >= SAVE_INTERVAL {
                    save_snapshot();
                    last_save = Instant::now();
                }
            }
        })
        .expect("failed to spawn folder size worker");
}

pub fn save_snapshot() {
    if let Some(path) = SNAPSHOT_PATH.get()
        && let Err(error) = ENGINE.save_if_changed(path)
    {
        warn!(?error, "could not write folder size snapshot");
    }
}

/// Called for every FSEvent batch the background loop receives.
pub fn note_events(events: &[FsEvent]) {
    for event in events {
        if event.flag.contains(EventFlag::HistoryDone) {
            continue;
        }
        let deep = event.flag.contains(EventFlag::MustScanSubDirs)
            || matches!(event.flag.scan_type(), ScanType::ReScan);
        ENGINE.note_change(&event.path, deep);
    }
}

/// Called after Move to Trash succeeded so the view updates without waiting
/// for the FSEvent round trip.
pub fn forget_paths<P: AsRef<Path>>(paths: &[P]) {
    for path in paths {
        ENGINE.forget(path.as_ref());
    }
}

fn parse_dir(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err("Folder path must be absolute".into());
    }
    let canonical = std::fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !canonical.is_dir() {
        return Err(format!("{} is not a folder", canonical.display()));
    }
    Ok(canonical)
}

#[tauri::command]
pub fn get_size_status() -> Status {
    ENGINE.status()
}

/// Scans `path`. Inside the current tree only that subtree is rescanned;
/// any other folder becomes the new root. Returns false when a scan is running.
#[tauri::command]
pub fn start_size_scan(path: String) -> Result<bool, String> {
    let activity = ACTIVITY.lock();
    if !activity.allowed(Instant::now()) {
        return Ok(false);
    }
    let path = parse_dir(&path)?;
    Ok(ENGINE.start_scan(path))
}

#[tauri::command]
pub fn cancel_size_scan() {
    ENGINE.cancel_scan();
}

#[tauri::command]
pub fn get_size_listing(path: String, nested: Option<bool>) -> Option<Listing> {
    ENGINE.listing(Path::new(&path), nested.unwrap_or(false))
}

#[tauri::command]
pub fn get_size_suggestions() -> Vec<Suggestion> {
    ENGINE.suggestions()
}

/// Free and total space of the volume holding `path` (the scanned root by default).
#[tauri::command]
pub fn get_volume_space(path: Option<String>) -> Option<VolumeSpace> {
    let path = path
        .map(PathBuf::from)
        .or_else(|| ENGINE.root_path())
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))?;
    let mut space = volume_space(&path)?;
    space.available_for_important_usage = important_usage_capacity(&path);
    Some(space)
}

/// `NSURLVolumeAvailableCapacityForImportantUsageKey`: the "available" figure
/// Finder shows, which includes purgeable space macOS can reclaim on demand.
fn important_usage_capacity(path: &Path) -> Option<u64> {
    use objc2::{rc::Retained, runtime::AnyObject};
    use objc2_foundation::{
        NSNumber, NSString, NSURL, NSURLVolumeAvailableCapacityForImportantUsageKey,
    };

    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    let mut value: Option<Retained<AnyObject>> = None;
    // SAFETY: the key is a valid NSURLResourceKey constant; `value` is an out-parameter.
    unsafe {
        url.getResourceValue_forKey_error(
            &mut value,
            NSURLVolumeAvailableCapacityForImportantUsageKey,
        )
        .ok()?;
    }
    let number = value?.downcast::<NSNumber>().ok()?;
    u64::try_from(number.longLongValue()).ok()
}

/// Size of a path from the live size tree, if it has been scanned.
pub fn size_of(path: &Path) -> Option<u64> {
    ENGINE.size_of(path)
}
