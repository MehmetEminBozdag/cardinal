//! Persistent log of Move to Trash operations.
//!
//! Every trash operation (context menu, app uninstall, orphaned leftovers) is
//! appended as one JSON line to `file-operations.jsonl` in the app's log dir,
//! with the original path of each item, where it went in the Trash when known
//! (items moved with administrator rights get a tagged name Finder can't
//! "Put Back"), its size and the outcome. The log rotates at 2 MB.

use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter};
use tracing::warn;

const FILE_NAME: &str = "file-operations.jsonl";
const ROTATE_BYTES: u64 = 2 * 1024 * 1024;
pub const LOGGED_EVENT: &str = "file-operation-logged";

static LOG_PATH: OnceCell<PathBuf> = OnceCell::new();
static APP: OnceCell<AppHandle> = OnceCell::new();
static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoggedItem {
    pub path: String,
    /// Location in the Trash, when known (items moved with admin rights).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trashed_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(default)]
    pub admin: bool,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    /// Context menu "Move to Trash" (Files and Sizes tabs).
    ContextMenu,
    AppUninstall,
    Orphans,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// Unix seconds.
    pub time: u64,
    pub source: Source,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    pub items: Vec<LoggedItem>,
    /// Installer receipts forgotten with `pkgutil --forget`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<String>,
}

impl LogEntry {
    pub fn now(source: Source, app: Option<String>, items: Vec<LoggedItem>) -> Self {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();
        Self {
            time,
            source,
            app,
            items,
            receipts: Vec::new(),
        }
    }
}

pub fn init(app: &AppHandle, log_dir: PathBuf) {
    let _ = LOG_PATH.set(log_dir.join(FILE_NAME));
    let _ = APP.set(app.clone());
}

/// Appends `entry` and notifies the UI. Failures are logged, never fatal.
pub fn record(entry: LogEntry) {
    if entry.items.is_empty() {
        return;
    }
    let Some(path) = LOG_PATH.get() else { return };
    if let Err(error) = append(path, &entry) {
        warn!(?error, "could not write file operation log");
        return;
    }
    if let Some(app) = APP.get() {
        let _ = app.emit(LOGGED_EVENT, ());
    }
}

pub(crate) fn append(path: &Path, entry: &LogEntry) -> std::io::Result<()> {
    let _guard = WRITE_LOCK.lock();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if std::fs::metadata(path).is_ok_and(|m| m.len() > ROTATE_BYTES) {
        std::fs::rename(path, path.with_extension("jsonl.1"))?;
    }
    let mut line = serde_json::to_string(entry).map_err(std::io::Error::other)?;
    line.push('\n');
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(line.as_bytes())
}

/// Newest first, at most `limit`, reading the rotated file when needed.
pub(crate) fn read(path: &Path, limit: usize) -> Vec<LogEntry> {
    let mut entries = Vec::new();
    for file in [path.to_path_buf(), path.with_extension("jsonl.1")] {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let mut batch: Vec<LogEntry> = text
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        batch.reverse();
        entries.extend(batch);
        if entries.len() >= limit {
            break;
        }
    }
    entries.truncate(limit);
    entries
}

#[tauri::command]
pub fn get_operation_log(limit: Option<usize>) -> Vec<LogEntry> {
    LOG_PATH
        .get()
        .map(|path| read(path, limit.unwrap_or(100).min(1_000)))
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_operation_log_path() -> Option<String> {
    LOG_PATH.get().map(|p| p.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(path: &str, ok: bool) -> LoggedItem {
        LoggedItem {
            path: path.into(),
            trashed_to: None,
            size: Some(10),
            admin: false,
            ok,
            error: (!ok).then(|| "denied".into()),
        }
    }

    #[test]
    fn appends_and_reads_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let first = LogEntry::now(Source::ContextMenu, None, vec![item("/a", true)]);
        let mut second = LogEntry::now(
            Source::AppUninstall,
            Some("Widget".into()),
            vec![item("/b", false)],
        );
        second.receipts = vec!["com.acme.widget.pkg".into()];
        append(&path, &first).unwrap();
        append(&path, &second).unwrap();
        let read_back = read(&path, 10);
        assert_eq!(read_back, vec![second, first]);
        assert_eq!(read(&path, 1).len(), 1);
    }

    #[test]
    fn rotates_large_logs_and_still_reads_them() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, "x".repeat((ROTATE_BYTES + 1) as usize)).unwrap();
        let entry = LogEntry::now(Source::Orphans, None, vec![item("/c", true)]);
        append(&path, &entry).unwrap();
        assert!(path.with_extension("jsonl.1").exists());
        assert_eq!(read(&path, 10), vec![entry]);
    }

    #[test]
    fn skips_corrupt_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let entry = LogEntry::now(Source::ContextMenu, None, vec![item("/d", true)]);
        append(&path, &entry).unwrap();
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(b"{not json\n").unwrap();
        assert_eq!(read(&path, 10), vec![entry]);
    }
}
