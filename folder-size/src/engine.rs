//! Thread-safe engine: owns the size tree, runs scans in the background and
//! applies file-system change notifications incrementally.

use crate::{
    persist,
    scan::{self, ScanContext},
    suggest::{self, Suggestion},
    tree::{Dir, Listing, MAX_ITEMS, SizeTree, list_items},
};
use parking_lot::Mutex;
use rayon::prelude::*;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tracing::{info, warn};

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Idle,
    Scanning,
    Ready,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub phase: Phase,
    /// Root of the tree being shown (or being scanned when there is none yet).
    pub root_path: Option<String>,
    pub scanning_path: Option<String>,
    pub scanned_files: u64,
    pub scanned_dirs: u64,
    pub scanned_bytes: u64,
    pub total_size: u64,
    pub total_files: u64,
    pub unreadable_count: u64,
    /// Unix seconds of the last full scan of the root.
    pub scanned_at: Option<u64>,
    pub last_scan_ms: Option<u64>,
    /// True while the tree comes from the on-disk snapshot and has not been
    /// rescanned in this session.
    pub from_snapshot: bool,
    /// Directories waiting for an incremental refresh.
    pub pending_updates: usize,
    /// Increases whenever the tree changes; the UI refetches on change.
    pub version: u64,
    /// Size of `~/.Trash` when it is inside the scanned tree. Items moved to
    /// the Trash only free disk space once the Trash is emptied.
    pub trash_size: Option<u64>,
}

struct Running {
    path: PathBuf,
    ctx: Arc<ScanContext>,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
struct State {
    tree: Option<SizeTree>,
    running: Option<Running>,
    from_snapshot: bool,
    last_scan_ms: Option<u64>,
    /// Bumped when the whole tree is replaced; stale incremental work is dropped.
    generation: u64,
    /// path -> deep (true: rescan subtree, false: re-read direct entries).
    dirty: BTreeMap<PathBuf, bool>,
    /// Tree changed since the last snapshot write.
    unsaved: bool,
}

pub struct SizeEngine {
    state: Mutex<State>,
    version: AtomicU64,
    home: PathBuf,
}

/// Above this many pending paths the whole tree is rescanned instead.
const MAX_DIRTY: usize = 5_000;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

impl SizeEngine {
    pub fn new(home: PathBuf) -> Self {
        Self {
            state: Mutex::new(State::default()),
            version: AtomicU64::new(0),
            home,
        }
    }

    fn bump(&self) {
        self.version.fetch_add(1, Ordering::SeqCst);
    }

    pub fn status(&self) -> Status {
        let st = self.state.lock();
        let (files, dirs, bytes) = st
            .running
            .as_ref()
            .map(|r| r.ctx.progress())
            .unwrap_or_default();
        let tree = st.tree.as_ref();
        Status {
            phase: if st.running.is_some() {
                Phase::Scanning
            } else if tree.is_some() {
                Phase::Ready
            } else {
                Phase::Idle
            },
            root_path: tree
                .map(|t| t.root_path.to_string_lossy().into_owned())
                .or_else(|| {
                    st.running
                        .as_ref()
                        .map(|r| r.path.to_string_lossy().into_owned())
                }),
            scanning_path: st
                .running
                .as_ref()
                .map(|r| r.path.to_string_lossy().into_owned()),
            scanned_files: files,
            scanned_dirs: dirs,
            scanned_bytes: bytes,
            total_size: tree.map(|t| t.root.size).unwrap_or_default(),
            total_files: tree.map(|t| t.root.file_count).unwrap_or_default(),
            unreadable_count: tree.map(|t| t.root.unreadable_count).unwrap_or_default(),
            scanned_at: tree.map(|t| t.scanned_at),
            last_scan_ms: st.last_scan_ms,
            from_snapshot: st.from_snapshot,
            pending_updates: st.dirty.len(),
            version: self.version.load(Ordering::SeqCst),
            trash_size: tree
                .and_then(|t| t.get(&self.home.join(".Trash")))
                .map(|d| d.size),
        }
    }

    /// Starts a background scan of `path`. If `path` is inside the current
    /// tree only that subtree is rescanned; otherwise the tree is replaced.
    /// Returns false when a scan is already running.
    pub fn start_scan(self: &Arc<Self>, path: PathBuf) -> bool {
        let cancel = Arc::new(AtomicBool::new(false));
        let ctx = Arc::new(ScanContext::new(&path, cancel.clone()));
        {
            let mut st = self.state.lock();
            if st.running.is_some() {
                return false;
            }
            st.running = Some(Running {
                path: path.clone(),
                ctx: ctx.clone(),
                cancel,
            });
        }
        self.bump();
        let engine = Arc::clone(self);
        std::thread::Builder::new()
            .name("folder-size-scan".into())
            .spawn(move || engine.run_scan(path, ctx))
            .is_ok()
    }

    fn run_scan(&self, path: PathBuf, ctx: Arc<ScanContext>) {
        let started = Instant::now();
        let result = scan::scan_tree(&path, &ctx);
        let elapsed = started.elapsed().as_millis() as u64;
        let mut st = self.state.lock();
        st.running = None;
        st.last_scan_ms = Some(elapsed);
        let Some(fresh) = result else {
            drop(st);
            self.bump();
            return;
        };
        info!(path = %path.display(), ms = elapsed, size = fresh.size, "folder size scan finished");
        let in_tree = st
            .tree
            .as_ref()
            .is_some_and(|t| t.contains_dir(&path) && t.root_path != path);
        if in_tree {
            st.tree.as_mut().unwrap().replace(&path, fresh);
        } else {
            st.tree = Some(SizeTree {
                root_path: path,
                root: fresh,
                scanned_at: now_secs(),
            });
            st.generation += 1;
            st.from_snapshot = false;
        }
        st.unsaved = true;
        drop(st);
        self.bump();
    }

    pub fn cancel_scan(&self) {
        if let Some(running) = &self.state.lock().running {
            running.cancel.store(true, Ordering::SeqCst);
        }
    }

    pub fn is_scanning(&self) -> bool {
        self.state.lock().running.is_some()
    }

    /// Directory listing for the size view. `nested` adds one more level for the treemap.
    pub fn listing(&self, path: &Path, nested: bool) -> Option<Listing> {
        let st = self.state.lock();
        let tree = st.tree.as_ref()?;
        let dir = tree.get(path)?;
        Some(Listing {
            path: path.to_string_lossy().into_owned(),
            root_path: tree.root_path.to_string_lossy().into_owned(),
            size: dir.size,
            file_count: dir.file_count,
            unreadable_count: dir.unreadable_count,
            items: list_items(dir, path, MAX_ITEMS, nested),
        })
    }

    pub fn suggestions(&self) -> Vec<Suggestion> {
        let st = self.state.lock();
        st.tree
            .as_ref()
            .map(|t| suggest::compute(&t.root, &t.root_path, &self.home))
            .unwrap_or_default()
    }

    /// Records a file-system change. Cheap; the work happens in [`Self::process_pending`].
    pub fn note_change(&self, path: &Path, deep: bool) {
        let mut st = self.state.lock();
        let Some(tree) = st.tree.as_ref() else { return };
        if !path.starts_with(&tree.root_path) {
            return;
        }
        let target = if deep || path == tree.root_path {
            path.to_path_buf()
        } else {
            // The item itself changed: its parent's direct entries need a re-read.
            path.parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| path.to_path_buf())
        };
        let entry = st.dirty.entry(target).or_insert(false);
        *entry |= deep;
    }

    /// Removes an item right away (used after Move to Trash succeeded).
    pub fn forget(&self, path: &Path) {
        let mut st = self.state.lock();
        let removed = st.tree.as_mut().is_some_and(|t| t.remove(path));
        if removed {
            st.unsaved = true;
            drop(st);
            self.bump();
        }
    }

    /// Applies pending changes. Returns the number of directories refreshed.
    pub fn process_pending(self: &Arc<Self>) -> usize {
        let (batch, generation, root) = {
            let mut st = self.state.lock();
            if st.running.is_some() || st.dirty.is_empty() {
                return 0;
            }
            let dirty = std::mem::take(&mut st.dirty);
            let Some(tree) = st.tree.as_ref() else {
                return 0;
            };
            let root = tree.root_path.clone();
            if dirty.len() > MAX_DIRTY {
                drop(st);
                warn!("too many pending folder-size changes; rescanning the whole tree");
                self.start_scan(root);
                return 0;
            }
            // Map each change to the deepest directory that exists in the tree,
            // and drop entries already covered by a deep rescan of an ancestor.
            let mut batch: BTreeMap<PathBuf, bool> = BTreeMap::new();
            for (path, deep) in dirty {
                let target = if tree.contains_dir(&path) {
                    path
                } else {
                    match tree.nearest_existing(&path) {
                        Some(p) => p,
                        None => continue,
                    }
                };
                *batch.entry(target).or_insert(false) |= deep;
            }
            let deep_roots: Vec<PathBuf> = batch
                .iter()
                .filter(|(_, d)| **d)
                .map(|(p, _)| p.clone())
                .collect();
            batch.retain(|p, _| !deep_roots.iter().any(|r| p != r && p.starts_with(r)));
            (batch, st.generation, root)
        };

        let count = batch.len();
        for (path, deep) in batch {
            if deep {
                self.refresh_subtree(&path, generation);
            } else {
                self.refresh_shallow(&path, generation, &root);
            }
        }
        if count > 0 {
            self.bump();
        }
        count
    }

    fn refresh_subtree(&self, path: &Path, generation: u64) {
        let ctx = ScanContext::new(path, Arc::new(AtomicBool::new(false)));
        let Some(fresh) = scan::scan_tree(path, &ctx) else {
            return;
        };
        let mut st = self.state.lock();
        if st.generation != generation {
            return;
        }
        if let Some(tree) = st.tree.as_mut() {
            if path == tree.root_path {
                tree.root = fresh;
                tree.root.name = path.to_string_lossy().into();
            } else {
                tree.replace(path, fresh);
            }
            st.unsaved = true;
        }
    }

    /// Re-reads one directory's direct entries; new subdirectories are scanned
    /// fully, vanished ones are dropped, unchanged subtrees are kept.
    fn refresh_shallow(&self, path: &Path, generation: u64, root: &Path) {
        let known: Vec<Box<str>> = {
            let st = self.state.lock();
            match st.tree.as_ref().and_then(|t| t.get(path)) {
                Some(dir) => dir.children.iter().map(|c| c.name.clone()).collect(),
                None => return,
            }
        };
        let ctx = ScanContext::new(root, Arc::new(AtomicBool::new(false)));
        let entries = scan::read_entries(path, &ctx);
        let new_children: Vec<Dir> = entries
            .subdirs
            .par_iter()
            .filter(|name| !known.contains(name))
            .map(|name| scan::scan_dir(&path.join(&**name), name.clone(), &ctx))
            .collect();
        let mut st = self.state.lock();
        if st.generation != generation {
            return;
        }
        if let Some(tree) = st.tree.as_mut()
            && tree.update(path, |dir| dir.apply_shallow(entries, new_children))
        {
            st.unsaved = true;
        }
    }

    /// Writes the snapshot if the tree changed since the last write.
    pub fn save_if_changed(&self, file: &Path) -> anyhow::Result<bool> {
        let tree = {
            let mut st = self.state.lock();
            if !st.unsaved || st.running.is_some() {
                return Ok(false);
            }
            let Some(tree) = st.tree.clone() else {
                return Ok(false);
            };
            st.unsaved = false;
            tree
        };
        persist::write(file, &tree)?;
        Ok(true)
    }

    /// Loads a snapshot written by [`Self::save_if_changed`]. The UI shows it
    /// at once; a fresh scan should follow because changes made while the app
    /// was closed are not in the snapshot.
    pub fn load(&self, file: &Path) -> anyhow::Result<bool> {
        let tree = persist::read(file)?;
        let mut st = self.state.lock();
        if st.tree.is_some() || st.running.is_some() {
            return Ok(false);
        }
        st.tree = Some(tree);
        st.from_snapshot = true;
        st.generation += 1;
        drop(st);
        self.bump();
        Ok(true)
    }

    pub fn root_path(&self) -> Option<PathBuf> {
        self.state.lock().tree.as_ref().map(|t| t.root_path.clone())
    }
}
