//! Installed applications and the files they leave behind.
//!
//! Discovery is read-only. Removal lives in the Tauri layer, which only acts on
//! paths produced by [`analyze`] for the same app.

use crate::scan::{ScanContext, scan_tree};
use rayon::prelude::*;
use serde::Serialize;
use std::{
    collections::{BTreeSet, HashMap},
    ffi::CString,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, atomic::AtomicBool},
};

/// Where to look. Injected so tests can use a fake home and `/Library`.
#[derive(Clone, Debug)]
pub struct Locations {
    pub app_dirs: Vec<PathBuf>,
    pub user_library: PathBuf,
    pub system_library: PathBuf,
    /// `DARWIN_USER_CACHE_DIR` / `DARWIN_USER_TEMP_DIR` style per-user folders.
    pub user_temp_dirs: Vec<PathBuf>,
}

impl Locations {
    pub fn system(home: &Path) -> Self {
        let user_temp_dirs = ["DARWIN_USER_CACHE_DIR", "DARWIN_USER_TEMP_DIR"]
            .iter()
            .filter_map(|key| {
                let out = Command::new("/usr/bin/getconf").arg(key).output().ok()?;
                let text = String::from_utf8(out.stdout).ok()?;
                let path = PathBuf::from(text.trim());
                path.is_dir().then_some(path)
            })
            .collect();
        Self {
            app_dirs: vec![PathBuf::from("/Applications"), home.join("Applications")],
            user_library: home.join("Library"),
            system_library: PathBuf::from("/Library"),
            user_temp_dirs,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBundle {
    pub name: String,
    pub path: String,
    pub bundle_id: String,
    pub version: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSummary {
    #[serde(flatten)]
    pub app: AppBundle,
    pub app_size: u64,
    /// Leftovers found in the standard Library folders (quick estimate).
    pub leftover_size: u64,
    pub running: bool,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Confidence {
    /// Named after the bundle id / app group, or installed by the app's package.
    Certain,
    /// Named after the app or its vendor, and no other installed app shares it.
    Likely,
    /// Could belong to another app; shown but not preselected.
    Possible,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum LeftoverKind {
    AppBundle,
    Support,
    Cache,
    Preferences,
    Container,
    SavedState,
    Logs,
    WebData,
    LaunchItem,
    Helper,
    Receipt,
    /// macOS's per-app "recent documents" list.
    RecentDocuments,
    Other,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Leftover {
    pub path: String,
    pub kind: LeftoverKind,
    pub confidence: Confidence,
    pub size: u64,
    pub needs_admin: bool,
    /// Launch agent/daemon plist: must be unloaded before removal.
    pub launch_item: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPlan {
    #[serde(flatten)]
    pub app: AppBundle,
    pub running: bool,
    pub items: Vec<Leftover>,
    /// Installer receipts (`pkgutil --pkgs`) to forget after removal.
    pub receipts: Vec<String>,
}

// ---------------------------------------------------------------------------
// Bundles

/// Reads name, bundle id and version from `Contents/Info.plist`.
pub fn read_bundle(path: &Path) -> Option<AppBundle> {
    let plist = path.join("Contents/Info.plist");
    let out = Command::new("/usr/bin/plutil")
        .args(["-convert", "json", "-o", "-"])
        .arg(&plist)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let bundle_id = json.get("CFBundleIdentifier")?.as_str()?.to_owned();
    let file_name = path.file_stem()?.to_string_lossy().into_owned();
    let name = json
        .get("CFBundleDisplayName")
        .or_else(|| json.get("CFBundleName"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .unwrap_or(file_name);
    let version = json
        .get("CFBundleShortVersionString")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    Some(AppBundle {
        name,
        path: path.to_string_lossy().into_owned(),
        bundle_id,
        version,
    })
}

/// Apple's own apps are part of the system and are never offered for removal.
pub fn is_system_app(app: &AppBundle) -> bool {
    app.bundle_id.starts_with("com.apple.") || app.path.starts_with("/System/")
}

/// `.app` bundles in the app folders (and one level of subfolders, e.g. "Adobe X/").
pub fn find_bundles(locations: &Locations) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in &locations.app_dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if !meta.is_dir() {
                continue;
            }
            if path.extension().is_some_and(|e| e == "app") {
                out.push(path);
            } else if let Ok(inner) = std::fs::read_dir(&path) {
                out.extend(
                    inner
                        .flatten()
                        .map(|e| e.path())
                        .filter(|p| p.extension().is_some_and(|e| e == "app")),
                );
            }
        }
    }
    out.sort();
    out
}

// ---------------------------------------------------------------------------
// Matching

/// `com.vendor` for `com.vendor.product`, if the id looks reverse-DNS.
pub fn vendor_prefix(bundle_id: &str) -> Option<String> {
    let parts: Vec<&str> = bundle_id.split('.').collect();
    (parts.len() >= 3 && parts[1].len() >= 3).then(|| format!("{}.{}", parts[0], parts[1]))
}

struct Matcher<'a> {
    bundle_id: String,
    name: String,
    vendor: Option<String>,
    vendor_shared: bool,
    groups: &'a [String],
}

impl Matcher<'_> {
    /// Classifies a directory entry name found in one of the Library folders.
    /// `vendor`: how much a vendor-prefix match (`com.vendor.*`) counts in this
    /// folder. Strong for launch items/helpers/group containers, which vendors
    /// name after themselves; weak elsewhere, where it may be another product.
    fn classify(&self, entry: &str, by_name: bool, vendor: VendorMatch) -> Option<Confidence> {
        let lower = entry.to_lowercase();
        let id = &self.bundle_id;
        let stem = lower
            .strip_suffix(".plist")
            .or_else(|| lower.strip_suffix(".sfl2"))
            .or_else(|| lower.strip_suffix(".sfl3"))
            .or_else(|| lower.strip_suffix(".savedstate"))
            .or_else(|| lower.strip_suffix(".binarycookies"))
            .unwrap_or(&lower);
        if stem == *id
            || lower.starts_with(&format!("{id}."))
            || lower.ends_with(&format!(".{id}"))
            || self.groups.contains(&lower)
        {
            return Some(Confidence::Certain);
        }
        if by_name && self.name.len() >= 3 && lower == self.name {
            return Some(Confidence::Likely);
        }
        if vendor != VendorMatch::No
            && let Some(prefix) = &self.vendor
            && (lower.starts_with(&format!("{prefix}.")) || lower.contains(&format!(".{prefix}.")))
        {
            return Some(if vendor == VendorMatch::Strong && !self.vendor_shared {
                Confidence::Likely
            } else {
                Confidence::Possible
            });
        }
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VendorMatch {
    No,
    Weak,
    Strong,
}

use VendorMatch::{No, Strong, Weak};

/// (relative folder, kind, match by app name, vendor match, system library too)
const LIBRARY_FOLDERS: &[(&str, LeftoverKind, bool, VendorMatch, bool)] = &[
    (
        "Application Support",
        LeftoverKind::Support,
        true,
        Weak,
        true,
    ),
    ("Caches", LeftoverKind::Cache, true, Weak, true),
    ("Preferences", LeftoverKind::Preferences, false, Weak, true),
    (
        "Preferences/ByHost",
        LeftoverKind::Preferences,
        false,
        No,
        false,
    ),
    ("Containers", LeftoverKind::Container, false, No, false),
    (
        "Group Containers",
        LeftoverKind::Container,
        false,
        Strong,
        false,
    ),
    (
        "Application Scripts",
        LeftoverKind::Container,
        false,
        No,
        false,
    ),
    (
        "Saved Application State",
        LeftoverKind::SavedState,
        false,
        No,
        false,
    ),
    ("Logs", LeftoverKind::Logs, true, Weak, true),
    ("HTTPStorages", LeftoverKind::WebData, false, No, false),
    ("WebKit", LeftoverKind::WebData, false, No, false),
    ("Cookies", LeftoverKind::WebData, false, No, false),
    (
        "LaunchAgents",
        LeftoverKind::LaunchItem,
        false,
        Strong,
        true,
    ),
    (
        "LaunchDaemons",
        LeftoverKind::LaunchItem,
        false,
        Strong,
        true,
    ),
    (
        "PrivilegedHelperTools",
        LeftoverKind::Helper,
        false,
        Strong,
        true,
    ),
    (
        "Application Support/com.apple.sharedfilelist/com.apple.LSSharedFileList.ApplicationRecentDocuments",
        LeftoverKind::RecentDocuments,
        false,
        No,
        false,
    ),
];

/// Directory listings of the Library folders, read once and shared by all apps.
pub struct LibrarySnapshot {
    folders: Vec<(PathBuf, LeftoverKind, bool, VendorMatch, Vec<String>)>,
}

impl LibrarySnapshot {
    pub fn read(locations: &Locations) -> Self {
        let mut folders = Vec::new();
        for &(rel, kind, by_name, vendor, system) in LIBRARY_FOLDERS {
            let mut roots = vec![locations.user_library.join(rel)];
            if system {
                roots.push(locations.system_library.join(rel));
            }
            for root in roots {
                if let Ok(entries) = std::fs::read_dir(&root) {
                    let names = entries
                        .flatten()
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .collect();
                    folders.push((root, kind, by_name, vendor, names));
                }
            }
        }
        for dir in &locations.user_temp_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                let names = entries
                    .flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect();
                folders.push((dir.clone(), LeftoverKind::Cache, false, No, names));
            }
        }
        Self { folders }
    }

    /// Every entry of every scanned Library folder: (folder, kind, entry name).
    pub fn entries(&self) -> impl Iterator<Item = (&Path, LeftoverKind, &str)> {
        self.folders.iter().flat_map(|(root, kind, _, _, names)| {
            names
                .iter()
                .map(move |n| (root.as_path(), *kind, n.as_str()))
        })
    }

    fn matches(&self, matcher: &Matcher) -> Vec<(PathBuf, LeftoverKind, Confidence)> {
        let mut out = Vec::new();
        for (root, kind, by_name, vendor, names) in &self.folders {
            for name in names {
                if let Some(conf) = matcher.classify(name, *by_name, *vendor) {
                    out.push((root.join(name), *kind, conf));
                }
            }
        }
        out
    }
}

fn matcher<'a>(app: &AppBundle, all_ids: &[String], groups: &'a [String]) -> Matcher<'a> {
    let bundle_id = app.bundle_id.to_lowercase();
    let vendor = vendor_prefix(&bundle_id);
    let vendor_shared = vendor.as_ref().is_some_and(|v| {
        all_ids.iter().any(|other| {
            let other = other.to_lowercase();
            other != bundle_id && other.starts_with(&format!("{v}."))
        })
    });
    Matcher {
        bundle_id,
        name: app.name.to_lowercase(),
        vendor,
        vendor_shared,
        groups,
    }
}

// ---------------------------------------------------------------------------
// Sizes, permissions, processes

pub fn path_size(path: &Path) -> u64 {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => {
            let ctx = ScanContext::new(path, Arc::new(AtomicBool::new(false)));
            scan_tree(path, &ctx).map(|d| d.size).unwrap_or_default()
        }
        Ok(meta) => {
            use std::os::unix::fs::MetadataExt;
            meta.blocks() * 512
        }
        Err(_) => 0,
    }
}

fn writable(path: &Path) -> bool {
    let Ok(c) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    unsafe { libc::access(c.as_ptr(), libc::W_OK) == 0 }
}

/// Moving `path` needs write access to its parent (and, for a directory, to itself).
pub fn needs_admin(path: &Path) -> bool {
    let parent_ok = path.parent().is_some_and(writable);
    let is_dir = std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir());
    !(parent_ok && (!is_dir || writable(path)))
}

/// Executable paths of all running processes the current user can see.
pub fn running_executables() -> Vec<PathBuf> {
    let count = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
    if count <= 0 {
        return Vec::new();
    }
    let mut pids = vec![0i32; count as usize * 2];
    let bytes = (pids.len() * size_of::<i32>()) as i32;
    let n = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), bytes) };
    let mut buf = vec![0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    pids.truncate(n.max(0) as usize);
    pids.into_iter()
        .filter_map(|pid| {
            let len = unsafe { libc::proc_pidpath(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
            (len > 0).then(|| PathBuf::from(std::ffi::OsStr::from_bytes(&buf[..len as usize])))
        })
        .collect()
}

pub fn is_running(app_path: &Path, running: &[PathBuf]) -> bool {
    running.iter().any(|exe| exe.starts_with(app_path))
}

// ---------------------------------------------------------------------------
// Signing groups and installer receipts

/// App group ids from the code signature's entitlements.
pub fn app_groups(app_path: &Path) -> Vec<String> {
    let Ok(out) = Command::new("/usr/bin/codesign")
        .args(["-d", "--entitlements", "-", "--xml"])
        .arg(app_path)
        .output()
    else {
        return Vec::new();
    };
    let xml = String::from_utf8_lossy(&out.stdout);
    let Some(start) = xml.find("com.apple.security.application-groups") else {
        return Vec::new();
    };
    let rest = &xml[start..];
    let Some(end) = rest.find("</array>") else {
        return Vec::new();
    };
    rest[..end]
        .split("<string>")
        .skip(1)
        .filter_map(|s| s.split("</string>").next())
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Package receipts that belong to the app and the files they installed
/// outside the app bundle (topmost paths that mention the app).
pub fn receipts(app: &AppBundle, vendor_shared: bool) -> Vec<(String, Confidence, Vec<PathBuf>)> {
    let Ok(out) = Command::new("/usr/sbin/pkgutil").arg("--pkgs").output() else {
        return Vec::new();
    };
    let id = app.bundle_id.to_lowercase();
    let vendor = vendor_prefix(&id);
    let name = app.name.to_lowercase();
    let mut result = Vec::new();
    for pkg in String::from_utf8_lossy(&out.stdout).lines() {
        let lower = pkg.to_lowercase();
        let confidence = if lower.contains(&id) {
            Confidence::Certain
        } else if vendor
            .as_ref()
            .is_some_and(|v| lower.starts_with(&format!("{v}.")))
        {
            if vendor_shared {
                Confidence::Possible
            } else {
                Confidence::Likely
            }
        } else {
            continue;
        };
        let files = receipt_files(pkg, app, &id, vendor.as_deref(), &name);
        result.push((pkg.to_owned(), confidence, files));
    }
    result
}

fn receipt_files(
    pkg: &str,
    app: &AppBundle,
    id: &str,
    vendor: Option<&str>,
    name: &str,
) -> Vec<PathBuf> {
    let info = Command::new("/usr/sbin/pkgutil")
        .args(["--pkg-info", pkg])
        .output();
    let mut base = PathBuf::from("/");
    if let Ok(info) = info {
        let text = String::from_utf8_lossy(&info.stdout);
        let field = |key: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(key))
                .map(|v| v.trim().to_owned())
        };
        if let Some(volume) = field("volume:") {
            base = PathBuf::from(volume);
        }
        if let Some(location) = field("location:") {
            base = base.join(location);
        }
    }
    let Ok(out) = Command::new("/usr/sbin/pkgutil")
        .args(["--files", pkg])
        .output()
    else {
        return Vec::new();
    };
    let app_path = Path::new(&app.path);
    let mentions = |p: &Path| {
        p.components().any(|c| {
            let c = c.as_os_str().to_string_lossy().to_lowercase();
            c.contains(id)
                || (name.len() >= 3 && c == name)
                || vendor.is_some_and(|v| c.starts_with(&format!("{v}.")))
        })
    };
    let mut picked: BTreeSet<PathBuf> = BTreeSet::new();
    for rel in String::from_utf8_lossy(&out.stdout).lines() {
        let path = base.join(rel);
        if path.starts_with(app_path)
            || !mentions(&path)
            || std::fs::symlink_metadata(&path).is_err()
        {
            continue;
        }
        if picked.iter().any(|p| path.starts_with(p)) {
            continue;
        }
        // Keep only the topmost mentioning ancestor.
        let mut top = path.clone();
        for ancestor in path.ancestors().skip(1) {
            if ancestor == Path::new("/") || !mentions(ancestor) || ancestor.starts_with(app_path) {
                break;
            }
            top = ancestor.to_path_buf();
        }
        picked.retain(|p| !p.starts_with(&top));
        picked.insert(top);
    }
    picked.into_iter().collect()
}

// ---------------------------------------------------------------------------
// Public entry points

/// Installed third-party apps with a quick leftover estimate, largest first.
pub fn list_apps(
    locations: &Locations,
    size_hint: &(dyn Fn(&Path) -> Option<u64> + Sync),
) -> Vec<AppSummary> {
    let bundles: Vec<AppBundle> = find_bundles(locations)
        .par_iter()
        .filter_map(|p| read_bundle(p))
        .filter(|a| !is_system_app(a))
        .collect();
    let ids: Vec<String> = bundles.iter().map(|a| a.bundle_id.clone()).collect();
    let library = LibrarySnapshot::read(locations);
    let running = running_executables();
    let size = |p: &Path| size_hint(p).unwrap_or_else(|| path_size(p));
    let mut apps: Vec<AppSummary> = bundles
        .into_par_iter()
        .map(|app| {
            let m = matcher(&app, &ids, &[]);
            let leftover_size = library
                .matches(&m)
                .iter()
                .filter(|(_, _, c)| *c != Confidence::Possible)
                .map(|(p, _, _)| size(p))
                .sum();
            AppSummary {
                running: is_running(Path::new(&app.path), &running),
                app_size: size(Path::new(&app.path)),
                leftover_size,
                app,
            }
        })
        .collect();
    apps.sort_by(|a, b| (b.app_size + b.leftover_size).cmp(&(a.app_size + a.leftover_size)));
    apps
}

/// Full removal plan for one app: bundle, Library leftovers (user and system),
/// app-group containers, installer receipts and their files, plus `extra`
/// paths found elsewhere (e.g. through Cardinal's index).
pub fn analyze(
    locations: &Locations,
    app_path: &Path,
    extra: &[PathBuf],
    size_hint: &(dyn Fn(&Path) -> Option<u64> + Sync),
) -> Option<AppPlan> {
    let app = read_bundle(app_path)?;
    if is_system_app(&app) {
        return None;
    }
    let ids: Vec<String> = find_bundles(locations)
        .iter()
        .filter_map(|p| read_bundle(p))
        .map(|a| a.bundle_id)
        .collect();
    let groups = app_groups(app_path);
    let m = matcher(&app, &ids, &groups);

    let mut found: HashMap<PathBuf, (LeftoverKind, Confidence)> = HashMap::new();
    let mut add = |path: PathBuf, kind: LeftoverKind, conf: Confidence| {
        let entry = found.entry(path).or_insert((kind, conf));
        if conf < entry.1 {
            *entry = (kind, conf);
        }
    };
    add(
        app_path.to_path_buf(),
        LeftoverKind::AppBundle,
        Confidence::Certain,
    );
    for (path, kind, conf) in LibrarySnapshot::read(locations).matches(&m) {
        add(path, kind, conf);
    }
    let mut receipt_ids = Vec::new();
    for (pkg, conf, files) in receipts(&app, m.vendor_shared) {
        if conf != Confidence::Possible {
            receipt_ids.push(pkg);
        }
        for file in files {
            add(file, LeftoverKind::Receipt, conf);
        }
    }
    let library_roots = [&locations.user_library, &locations.system_library];
    for path in extra {
        if path.starts_with(app_path) {
            continue;
        }
        let in_library = library_roots.iter().any(|r| path.starts_with(r))
            || locations.user_temp_dirs.iter().any(|r| path.starts_with(r));
        add(
            path.clone(),
            LeftoverKind::Other,
            if in_library {
                Confidence::Likely
            } else {
                Confidence::Possible
            },
        );
    }

    // Drop entries nested inside another selected entry.
    let mut paths: Vec<PathBuf> = found.keys().cloned().collect();
    paths.sort();
    let mut kept: Vec<PathBuf> = Vec::new();
    for path in paths {
        if !kept.iter().any(|k| path.starts_with(k)) {
            kept.push(path);
        }
    }

    let mut items: Vec<Leftover> = kept
        .par_iter()
        .map(|path| {
            let (kind, confidence) = found[path];
            let launch_item =
                kind == LeftoverKind::LaunchItem && path.extension().is_some_and(|e| e == "plist");
            Leftover {
                path: path.to_string_lossy().into_owned(),
                kind,
                confidence,
                size: size_hint(path).unwrap_or_else(|| path_size(path)),
                needs_admin: needs_admin(path),
                launch_item,
            }
        })
        .collect();
    items.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then(a.confidence.cmp(&b.confidence))
            .then(b.size.cmp(&a.size))
    });
    Some(AppPlan {
        running: is_running(app_path, &running_executables()),
        app,
        items,
        receipts: receipt_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(groups: &[String], shared: bool) -> Matcher<'_> {
        Matcher {
            bundle_id: "com.acme.widget".into(),
            name: "widget".into(),
            vendor: Some("com.acme".into()),
            vendor_shared: shared,
            groups,
        }
    }

    #[test]
    fn classifies_entries() {
        let groups = vec!["abcde12345.com.acme.shared".to_owned()];
        let m = m(&groups, false);
        assert_eq!(
            m.classify("com.acme.widget", false, No),
            Some(Confidence::Certain)
        );
        assert_eq!(
            m.classify("com.acme.widget.plist", false, No),
            Some(Confidence::Certain)
        );
        assert_eq!(
            m.classify("com.acme.widget.savedState", false, No),
            Some(Confidence::Certain)
        );
        assert_eq!(
            m.classify("com.acme.widget.ShareExtension", false, No),
            Some(Confidence::Certain)
        );
        assert_eq!(
            m.classify("TEAM.com.acme.widget", false, No),
            Some(Confidence::Certain)
        );
        assert_eq!(
            m.classify("ABCDE12345.com.acme.shared", false, Strong),
            Some(Confidence::Certain)
        );
        assert_eq!(m.classify("Widget", true, No), Some(Confidence::Likely));
        assert_eq!(m.classify("Widget", false, No), None);
        assert_eq!(
            m.classify("com.acme.updater.plist", false, Strong),
            Some(Confidence::Likely)
        );
        assert_eq!(
            m.classify("com.acme.widgetpro", false, Weak),
            Some(Confidence::Possible)
        );
        assert_eq!(m.classify("com.acme.widgetpro", false, No), None);
        assert_eq!(m.classify("com.other.widget2", true, Strong), None);
    }

    #[test]
    fn shared_vendor_is_only_possible() {
        let m = m(&[], true);
        assert_eq!(
            m.classify("com.acme.updater.plist", false, Strong),
            Some(Confidence::Possible)
        );
    }

    #[test]
    fn vendor_prefix_needs_reverse_dns() {
        assert_eq!(
            vendor_prefix("com.spotify.client").as_deref(),
            Some("com.spotify")
        );
        assert_eq!(vendor_prefix("com.x.app"), None);
        assert_eq!(vendor_prefix("single"), None);
    }

    #[test]
    fn finds_this_process_running() {
        let exe = std::env::current_exe().unwrap();
        let running = running_executables();
        assert!(is_running(exe.parent().unwrap(), &running));
    }
}
