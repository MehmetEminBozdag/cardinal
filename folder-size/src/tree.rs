//! In-memory size tree and the operations that keep it current.

use crate::scan::Entries;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    pub name: Box<str>,
    pub size: u64,
}

/// One directory. `size`, `file_count` and `unreadable_count` are subtree
/// totals; the other fields describe only the directory's direct entries.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Dir {
    pub name: Box<str>,
    pub children: Vec<Dir>,
    /// Direct files at or above the big-file threshold, largest first.
    pub files: Vec<FileEntry>,
    pub files_size: u64,
    pub direct_files: u64,
    pub unreadable: bool,
    pub size: u64,
    pub file_count: u64,
    pub unreadable_count: u64,
}

impl Dir {
    pub fn named(name: Box<str>) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }

    pub(crate) fn from_entries(name: Box<str>, entries: Entries) -> Self {
        Self {
            name,
            files: entries.big_files,
            files_size: entries.files_size,
            direct_files: entries.direct_files,
            unreadable: entries.unreadable,
            ..Self::default()
        }
    }

    /// Sorts children by size and recomputes totals from direct fields.
    pub fn finish(&mut self) {
        self.children.sort_unstable_by(|a, b| b.size.cmp(&a.size));
        self.recompute();
    }

    pub fn recompute(&mut self) {
        let mut size = self.files_size;
        let mut files = self.direct_files;
        let mut unreadable = u64::from(self.unreadable);
        for child in &self.children {
            size += child.size;
            files += child.file_count;
            unreadable += child.unreadable_count;
        }
        self.size = size;
        self.file_count = files;
        self.unreadable_count = unreadable;
    }

    pub fn small_files_size(&self) -> u64 {
        self.files_size
            .saturating_sub(self.files.iter().map(|f| f.size).sum::<u64>())
    }

    pub fn small_files_count(&self) -> u64 {
        self.direct_files.saturating_sub(self.files.len() as u64)
    }

    pub fn child(&self, name: &str) -> Option<&Dir> {
        self.children.iter().find(|c| &*c.name == name)
    }

    /// Replaces direct-entry data after a shallow re-read. Existing child
    /// subtrees whose names still exist are kept; `new_children` are added.
    pub(crate) fn apply_shallow(&mut self, entries: Entries, new_children: Vec<Dir>) {
        let keep: std::collections::HashSet<&str> = entries.subdirs.iter().map(|s| &**s).collect();
        self.children.retain(|c| keep.contains(&*c.name));
        for child in new_children {
            if self.child(&child.name).is_none() {
                self.children.push(child);
            }
        }
        self.files = entries.big_files;
        self.files_size = entries.files_size;
        self.direct_files = entries.direct_files;
        self.unreadable = entries.unreadable;
        self.finish();
    }
}

/// A scanned root plus its tree.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SizeTree {
    pub root_path: PathBuf,
    pub root: Dir,
    /// Unix seconds of the last full scan.
    pub scanned_at: u64,
}

impl SizeTree {
    /// Path components of `path` relative to the root, or `None` when outside.
    pub fn relative(&self, path: &Path) -> Option<Vec<String>> {
        let rest = path.strip_prefix(&self.root_path).ok()?;
        let mut out = Vec::new();
        for comp in rest.components() {
            match comp {
                Component::Normal(name) => out.push(name.to_string_lossy().into_owned()),
                Component::CurDir => {}
                _ => return None,
            }
        }
        Some(out)
    }

    pub fn get(&self, path: &Path) -> Option<&Dir> {
        let mut dir = &self.root;
        for name in self.relative(path)? {
            dir = dir.child(&name)?;
        }
        Some(dir)
    }

    pub fn contains_dir(&self, path: &Path) -> bool {
        self.get(path).is_some()
    }

    /// Runs `f` on the directory at `path` and refreshes the totals (and child
    /// order) of every ancestor. Returns false if the directory is not in the tree.
    pub fn update(&mut self, path: &Path, f: impl FnOnce(&mut Dir)) -> bool {
        let Some(comps) = self.relative(path) else {
            return false;
        };
        fn walk(dir: &mut Dir, comps: &[String], f: impl FnOnce(&mut Dir)) -> bool {
            match comps.split_first() {
                None => {
                    f(dir);
                    dir.finish();
                    true
                }
                Some((head, rest)) => {
                    let Some(child) = dir.children.iter_mut().find(|c| &*c.name == head) else {
                        return false;
                    };
                    let found = walk(child, rest, f);
                    if found {
                        dir.finish();
                    }
                    found
                }
            }
        }
        walk(&mut self.root, &comps, f)
    }

    /// Replaces the subtree at `path` with a freshly scanned one.
    pub fn replace(&mut self, path: &Path, mut fresh: Dir) -> bool {
        self.update(path, |dir| {
            fresh.name = std::mem::take(&mut dir.name);
            *dir = fresh;
        })
    }

    /// Removes a directory or a big file from the tree (e.g. after Move to Trash).
    pub fn remove(&mut self, path: &Path) -> bool {
        let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
            return false;
        };
        let name = name.to_string_lossy();
        let mut removed = false;
        self.update(parent, |dir| {
            if let Some(pos) = dir.children.iter().position(|c| *c.name == *name) {
                dir.children.remove(pos);
                removed = true;
            } else if let Some(pos) = dir.files.iter().position(|f| *f.name == *name) {
                let file = dir.files.remove(pos);
                dir.files_size = dir.files_size.saturating_sub(file.size);
                dir.direct_files = dir.direct_files.saturating_sub(1);
                removed = true;
            }
        });
        removed
    }

    /// Deepest directory of the tree that is `path` or one of its ancestors.
    pub fn nearest_existing(&self, path: &Path) -> Option<PathBuf> {
        let comps = self.relative(path)?;
        let mut dir = &self.root;
        let mut out = self.root_path.clone();
        for name in comps {
            match dir.child(&name) {
                Some(child) => {
                    dir = child;
                    out.push(&name);
                }
                None => break,
            }
        }
        Some(out)
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    Dir,
    File,
    Small,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    pub name: String,
    pub path: String,
    pub kind: ItemKind,
    pub size: u64,
    pub file_count: u64,
    pub unreadable_count: u64,
    /// Second level for the treemap (only when requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<ListItem>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub path: String,
    pub root_path: String,
    pub size: u64,
    pub file_count: u64,
    pub unreadable_count: u64,
    pub items: Vec<ListItem>,
}

/// Maximum entries returned per directory level.
pub const MAX_ITEMS: usize = 2_000;
pub const MAX_NESTED_ITEMS: usize = 60;

pub fn list_items(dir: &Dir, path: &Path, limit: usize, nested: bool) -> Vec<ListItem> {
    let join = |name: &str| path.join(name).to_string_lossy().into_owned();
    let mut items: Vec<ListItem> = Vec::with_capacity(dir.children.len() + dir.files.len() + 1);
    for child in &dir.children {
        let child_path = path.join(&*child.name);
        items.push(ListItem {
            name: child.name.to_string(),
            path: child_path.to_string_lossy().into_owned(),
            kind: ItemKind::Dir,
            size: child.size,
            file_count: child.file_count,
            unreadable_count: child.unreadable_count,
            children: nested.then(|| list_items(child, &child_path, MAX_NESTED_ITEMS, false)),
        });
    }
    for file in &dir.files {
        items.push(ListItem {
            name: file.name.to_string(),
            path: join(&file.name),
            kind: ItemKind::File,
            size: file.size,
            file_count: 1,
            unreadable_count: 0,
            children: None,
        });
    }
    let small = dir.small_files_count();
    if small > 0 {
        items.push(ListItem {
            name: String::new(),
            path: path.to_string_lossy().into_owned(),
            kind: ItemKind::Small,
            size: dir.small_files_size(),
            file_count: small,
            unreadable_count: 0,
            children: None,
        });
    }
    items.sort_by(|a, b| b.size.cmp(&a.size));
    items.truncate(limit);
    items
}
