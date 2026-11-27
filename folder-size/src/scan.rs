//! Parallel directory scanner measuring *allocated* size (what `du` reports).
//!
//! On APFS/HFS+ it uses `getattrlistbulk(2)`, which returns name, type, inode,
//! link count and allocated size for many entries in a single syscall. Other
//! file systems fall back to `readdir` + `lstat`.

use crate::tree::{Dir, FileEntry};
use parking_lot::Mutex;
use rayon::prelude::*;
use std::{
    collections::HashSet,
    ffi::{CStr, CString, OsStr},
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

/// Files at or above this allocated size are kept individually in the tree;
/// smaller ones are only counted (keeps memory flat on million-file trees).
pub const BIG_FILE_THRESHOLD: u64 = 1 << 20;

/// Paths skipped when scanning `/`: other volumes and firmlink duplicates.
const SYSTEM_ROOT_EXCLUDES: &[&str] = &[
    "/System/Volumes",
    "/Volumes",
    "/dev",
    "/net",
    "/home",
    "/Network",
    "/cores",
];

// Not exported by the libc crate.
const ATTR_CMN_ERROR: libc::attrgroup_t = 0x2000_0000;
const VREG: u32 = 1;
const VDIR: u32 = 2;
const BULK_BUFFER_SIZE: usize = 128 * 1024;

/// Shared state of one scan: cancellation, progress counters, hard-link set.
pub struct ScanContext {
    system_root: bool,
    root_dev: Option<libc::dev_t>,
    cancel: Arc<AtomicBool>,
    pub files: AtomicU64,
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    seen_links: Mutex<HashSet<(libc::dev_t, u64)>>,
}

impl ScanContext {
    pub fn new(root: &Path, cancel: Arc<AtomicBool>) -> Self {
        let system_root = root == Path::new("/");
        let root_dev = std::fs::symlink_metadata(root)
            .ok()
            .map(|m| m.dev() as libc::dev_t);
        Self {
            system_root,
            root_dev,
            cancel,
            files: AtomicU64::new(0),
            dirs: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            seen_links: Mutex::new(HashSet::new()),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn progress(&self) -> (u64, u64, u64) {
        (
            self.files.load(Ordering::Relaxed),
            self.dirs.load(Ordering::Relaxed),
            self.bytes.load(Ordering::Relaxed),
        )
    }

    fn first_link(&self, dev: libc::dev_t, ino: u64) -> bool {
        self.seen_links.lock().insert((dev, ino))
    }
}

/// Direct entries of one directory.
#[cfg(test)]
mod cancellation_tests {
    use super::*;

    #[test]
    fn cancelled_direct_read_does_not_enumerate_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("file"), b"test").unwrap();
        let ctx = ScanContext::new(dir.path(), Arc::new(AtomicBool::new(true)));
        assert_eq!(read_entries(dir.path(), &ctx).direct_files, 0);
    }
}

/// Direct entries of one directory.
#[derive(Debug, Default)]
pub struct Entries {
    pub unreadable: bool,
    /// Directory lives on another volume and was not entered.
    pub skipped: bool,
    pub files_size: u64,
    pub direct_files: u64,
    pub big_files: Vec<FileEntry>,
    pub subdirs: Vec<Box<str>>,
}

/// Scans `path` recursively. Returns `None` when cancelled.
pub fn scan_tree(path: &Path, ctx: &ScanContext) -> Option<Dir> {
    let name: Box<str> = path.to_string_lossy().into();
    let dir = scan_dir(path, name, ctx);
    (!ctx.is_cancelled()).then_some(dir)
}

/// Scans one directory and everything below it (in parallel).
pub fn scan_dir(path: &Path, name: Box<str>, ctx: &ScanContext) -> Dir {
    if ctx.is_cancelled() {
        return Dir::named(name);
    }
    let entries = read_entries(path, ctx);
    let children: Vec<Dir> = entries
        .subdirs
        .par_iter()
        .map(|sub| scan_dir(&path.join(&**sub), sub.clone(), ctx))
        .collect();
    let mut dir = Dir::from_entries(name, entries);
    dir.children = children;
    dir.finish();
    dir
}

/// Reads the direct entries of `path` (no recursion).
pub fn read_entries(path: &Path, ctx: &ScanContext) -> Entries {
    let mut out = Entries::default();
    if ctx.is_cancelled() {
        return out;
    }
    let Ok(cpath) = CString::new(path.as_os_str().as_bytes()) else {
        out.unreadable = true;
        return out;
    };
    let fd = unsafe {
        libc::open(
            cpath.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        out.unreadable = true;
        return out;
    }
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let dev = if unsafe { libc::fstat(fd, &mut st) } == 0 {
        st.st_dev
    } else {
        0
    };
    if !ctx.system_root && ctx.root_dev.is_some_and(|root| root != dev) {
        unsafe { libc::close(fd) };
        out.skipped = true;
        return out;
    }

    let bulk_ok = read_bulk(fd, dev, path, ctx, &mut out);
    unsafe { libc::close(fd) };
    if !bulk_ok {
        out = Entries::default();
        read_posix(path, dev, ctx, &mut out);
    }
    out.big_files.sort_unstable_by(|a, b| b.size.cmp(&a.size));
    ctx.files.fetch_add(out.direct_files, Ordering::Relaxed);
    ctx.bytes.fetch_add(out.files_size, Ordering::Relaxed);
    ctx.dirs.fetch_add(1, Ordering::Relaxed);
    out
}

fn push_dir(path: &Path, name: &[u8], ctx: &ScanContext, out: &mut Entries) {
    let name = OsStr::from_bytes(name);
    if ctx.system_root {
        let full = path.join(name);
        if SYSTEM_ROOT_EXCLUDES.iter().any(|ex| full == Path::new(ex)) {
            return;
        }
    }
    out.subdirs.push(name.to_string_lossy().into());
}

#[allow(clippy::too_many_arguments)]
fn push_file(
    name: &[u8],
    alloc: u64,
    regular: bool,
    links: u32,
    dev: libc::dev_t,
    ino: u64,
    ctx: &ScanContext,
    out: &mut Entries,
) {
    // A hard-linked file is counted once, at the first path that reaches it.
    let alloc = if regular && links > 1 && !ctx.first_link(dev, ino) {
        0
    } else {
        alloc
    };
    out.direct_files += 1;
    out.files_size += alloc;
    if alloc >= BIG_FILE_THRESHOLD {
        out.big_files.push(FileEntry {
            name: OsStr::from_bytes(name).to_string_lossy().into(),
            size: alloc,
        });
    }
}

/// Returns false when the file system does not support `getattrlistbulk`.
fn read_bulk(fd: i32, dev: libc::dev_t, path: &Path, ctx: &ScanContext, out: &mut Entries) -> bool {
    let mut attrs: libc::attrlist = unsafe { std::mem::zeroed() };
    attrs.bitmapcount = libc::ATTR_BIT_MAP_COUNT;
    attrs.commonattr = libc::ATTR_CMN_RETURNED_ATTRS
        | ATTR_CMN_ERROR
        | libc::ATTR_CMN_NAME
        | libc::ATTR_CMN_OBJTYPE
        | libc::ATTR_CMN_FILEID;
    attrs.fileattr = libc::ATTR_FILE_LINKCOUNT | libc::ATTR_FILE_ALLOCSIZE;

    let mut buf = vec![0u8; BULK_BUFFER_SIZE];
    let mut first = true;
    loop {
        if ctx.is_cancelled() {
            return true;
        }
        let count = unsafe {
            libc::getattrlistbulk(
                fd,
                (&mut attrs as *mut libc::attrlist).cast(),
                buf.as_mut_ptr().cast(),
                buf.len(),
                0,
            )
        };
        if count < 0 {
            if first {
                return false;
            }
            out.unreadable = true;
            return true;
        }
        first = false;
        if count == 0 {
            return true;
        }
        let mut entry = buf.as_ptr();
        for _ in 0..count {
            unsafe {
                let len = (entry as *const u32).read_unaligned() as usize;
                let mut p = entry.add(4);
                let returned = (p as *const libc::attribute_set_t).read_unaligned();
                p = p.add(size_of::<libc::attribute_set_t>());

                let mut error = 0u32;
                if returned.commonattr & ATTR_CMN_ERROR != 0 {
                    error = (p as *const u32).read_unaligned();
                    p = p.add(4);
                }
                let mut name: &[u8] = &[];
                if returned.commonattr & libc::ATTR_CMN_NAME != 0 {
                    let offset = (p as *const i32).read_unaligned();
                    name = CStr::from_ptr(p.offset(offset as isize).cast()).to_bytes();
                    p = p.add(size_of::<libc::attrreference_t>());
                }
                let mut obj_type = 0u32;
                if returned.commonattr & libc::ATTR_CMN_OBJTYPE != 0 {
                    obj_type = (p as *const u32).read_unaligned();
                    p = p.add(4);
                }
                let mut ino = 0u64;
                if returned.commonattr & libc::ATTR_CMN_FILEID != 0 {
                    ino = (p as *const u64).read_unaligned();
                    p = p.add(8);
                }
                let mut links = 1u32;
                if returned.fileattr & libc::ATTR_FILE_LINKCOUNT != 0 {
                    links = (p as *const u32).read_unaligned();
                    p = p.add(4);
                }
                let mut alloc = 0u64;
                if returned.fileattr & libc::ATTR_FILE_ALLOCSIZE != 0 {
                    alloc = (p as *const i64).read_unaligned().max(0) as u64;
                }

                if error == 0 && !name.is_empty() {
                    if obj_type == VDIR {
                        push_dir(path, name, ctx, out);
                    } else {
                        push_file(name, alloc, obj_type == VREG, links, dev, ino, ctx, out);
                    }
                }
                entry = entry.add(len);
            }
        }
    }
}

fn read_posix(path: &Path, dev: libc::dev_t, ctx: &ScanContext, out: &mut Entries) {
    let Ok(iter) = std::fs::read_dir(path) else {
        out.unreadable = true;
        return;
    };
    for entry in iter.flatten() {
        if ctx.is_cancelled() {
            break;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let name = entry.file_name();
        if meta.is_dir() {
            push_dir(path, name.as_bytes(), ctx, out);
        } else {
            push_file(
                name.as_bytes(),
                meta.blocks() * 512,
                meta.is_file(),
                meta.nlink() as u32,
                dev,
                meta.ino(),
                ctx,
                out,
            );
        }
    }
}

/// Full path of a child entry.
pub fn child_path(parent: &Path, name: &str) -> PathBuf {
    parent.join(name)
}
