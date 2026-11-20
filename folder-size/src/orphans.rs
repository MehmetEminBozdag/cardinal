//! Leftovers of apps that are no longer installed ("orphans").
//!
//! An app removed by dragging it to the Trash leaves its Library data behind,
//! and nothing points back to it any more. This finds:
//!
//! * Library entries named after a bundle id (`com.vendor.product…`) that no
//!   installed app, no app known to Spotlight and no vendor of an installed
//!   app accounts for;
//! * folders in `~/Library/Application Support` that only contain copies of
//!   such apps (`Foo.app`, `Foo.app.backup`), e.g. an updater's old backups;
//! * Application Support / Caches / Logs folders whose name matches one of the
//!   orphaned products (e.g. `codeburn-desktop` next to `org.x.codeburn-…`).
//!
//! Everything is reported as [`Confidence::Possible`]: nothing is preselected.

use crate::apps::{
    AppBundle, Confidence, Leftover, LeftoverKind, LibrarySnapshot, Locations, find_bundles,
    needs_admin, path_size, read_bundle, vendor_prefix,
};
use rayon::prelude::*;
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanGroup {
    /// Grouping key (normalised product name).
    pub key: String,
    /// Best human-readable name (app name from a leftover bundle, or an id).
    pub label: String,
    pub size: u64,
    pub items: Vec<Leftover>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanReport {
    pub groups: Vec<OrphanGroup>,
    pub total: u64,
}

/// First components that mark a reverse-DNS bundle id.
const ID_ROOTS: &[&str] = &[
    "com", "org", "net", "io", "local", "de", "me", "app", "dev", "co", "uk", "fr", "ru", "jp",
    "cn", "us", "ch", "nl", "se", "at", "be", "ca", "au", "it", "es", "tr", "info", "pl", "ai",
];
const MIN_TOKEN_LEN: usize = 5;
/// System components that use bundle-id style names but are not apps.
const SYSTEM_PREFIXES: &[&str] = &[
    "com.apple.",
    "org.cups.",
    "org.swift.",
    "com.openssh.",
    "org.openbsd.",
    "org.python.",
    "org.macports.",
    "sh.brew.",
    "org.homebrew.",
];

/// Lower-case alphanumerics only: "CodeBurn Offline" -> "codeburnoffline".
fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// Bundle id an entry name refers to, if it looks like one.
pub fn entry_id(name: &str) -> Option<String> {
    let mut lower = name.to_lowercase();
    for suffix in [
        ".plist",
        ".savedstate",
        ".binarycookies",
        ".sfl2",
        ".sfl3",
        ".sfl4",
    ] {
        if let Some(stripped) = lower.strip_suffix(suffix) {
            lower = stripped.to_owned();
            break;
        }
    }
    if let Some(stripped) = lower.strip_prefix("group.") {
        lower = stripped.to_owned();
    }
    // Team-id prefixed group containers: "ABCDE12345.com.vendor.shared".
    if let Some((head, rest)) = lower.split_once('.')
        && head.len() == 10
        && head.chars().all(|c| c.is_ascii_alphanumeric())
        && head.chars().any(|c| c.is_ascii_digit())
    {
        lower = rest.to_owned();
    }
    let parts: Vec<&str> = lower.split('.').collect();
    (parts.len() >= 3
        && ID_ROOTS.contains(&parts[0])
        && parts[1].len() >= 2
        && !parts[2].is_empty())
    .then_some(lower)
}

/// Product token of an id: `local.x.codeburn-offline.helper` -> "codeburn".
fn product_token(id: &str) -> String {
    let third = id.split('.').nth(2).unwrap_or(id);
    let first = third.split(['-', '_']).next().unwrap_or(third);
    normalize(first)
}

/// Bundle ids of every app Spotlight knows about, except copies kept inside
/// `skip` folders (backups in Application Support, the Trash).
pub fn spotlight_app_ids(skip: &[PathBuf]) -> Vec<String> {
    let Ok(out) = Command::new("/usr/bin/mdfind")
        .args([
            "-attr",
            "kMDItemCFBundleIdentifier",
            "kMDItemContentType == 'com.apple.application-bundle'",
        ])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let (path, id) = line.rsplit_once("kMDItemCFBundleIdentifier = ")?;
            let path = Path::new(path.trim_end());
            let id = id.trim();
            (!skip.iter().any(|s| path.starts_with(s)) && id != "(null)").then(|| id.to_lowercase())
        })
        .collect()
}

struct Known {
    ids: HashSet<String>,
    vendors: HashSet<String>,
    /// Normalised names/ids of installed apps, to keep name matches away from them.
    names: Vec<String>,
}

impl Known {
    fn new(installed: &[AppBundle], extra_ids: &[String]) -> Self {
        let mut ids: HashSet<String> = extra_ids.iter().map(|s| s.to_lowercase()).collect();
        let mut vendors = HashSet::new();
        let mut names = Vec::new();
        for app in installed {
            let id = app.bundle_id.to_lowercase();
            if !id.starts_with("com.apple.")
                && let Some(v) = vendor_prefix(&id)
            {
                vendors.insert(v);
            }
            names.push(normalize(&app.name));
            names.push(normalize(&id));
            ids.insert(id);
        }
        Self {
            ids,
            vendors,
            names,
        }
    }

    /// Whether an installed app (or the system) explains `id`. With
    /// `by_vendor`, any id of an installed app's vendor counts too, except in
    /// personal namespaces (`local.<user>.…`) where unrelated tools share it.
    fn accounts_for(&self, id: &str, by_vendor: bool) -> bool {
        SYSTEM_PREFIXES.iter().any(|p| id.starts_with(p))
            || self.ids.iter().any(|k| {
                id == k || id.starts_with(&format!("{k}.")) || k.starts_with(&format!("{id}."))
            })
            || (by_vendor
                && !id.starts_with("local.")
                && vendor_prefix(id).is_some_and(|v| self.vendors.contains(&v)))
    }

    fn mentions(&self, token: &str) -> bool {
        self.names.iter().any(|n| n.contains(token))
    }
}

/// `.app` / `.app.backup` bundles below `dir` (not descending into bundles).
fn app_copies(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.ends_with(".app") || name.ends_with(".app.backup") {
            out.push(path);
        } else {
            app_copies(&path, depth - 1, out);
        }
    }
}

pub fn find_orphans(
    locations: &Locations,
    extra_known_ids: &[String],
    size_hint: &(dyn Fn(&Path) -> Option<u64> + Sync),
) -> OrphanReport {
    let installed: Vec<AppBundle> = find_bundles(locations)
        .iter()
        .filter_map(|p| read_bundle(p))
        .collect();
    let known = Known::new(&installed, extra_known_ids);
    let snapshot = LibrarySnapshot::read(locations);

    // key -> (label, items)
    let mut groups: BTreeMap<String, (String, Vec<(PathBuf, LeftoverKind)>)> = BTreeMap::new();
    // Groups backed by a leftover app copy: the only ones trusted for name matches.
    let mut with_copies: HashSet<String> = HashSet::new();

    // 1. Entries named after a bundle id nobody accounts for.
    for (root, kind, name) in snapshot.entries() {
        let Some(id) = entry_id(name) else { continue };
        if known.accounts_for(&id, true) {
            continue;
        }
        let key = product_token(&id);
        if key.is_empty() || known.mentions(&key) {
            continue;
        }
        groups
            .entry(key)
            .or_insert_with(|| (id.clone(), Vec::new()))
            .1
            .push((root.join(name), kind));
    }

    // 2. Application Support folders that only hold copies of uninstalled apps.
    let support = locations.user_library.join("Application Support");
    if let Ok(entries) = std::fs::read_dir(&support) {
        for entry in entries.flatten() {
            let top = entry.path();
            let mut copies = Vec::new();
            app_copies(&top, 4, &mut copies);
            // Ignore copies whose Info.plist can't be read; one readable copy is enough.
            let bundles: Vec<AppBundle> = copies.iter().filter_map(|p| read_bundle(p)).collect();
            if bundles.is_empty() {
                continue;
            }
            if bundles
                .iter()
                .any(|b| known.accounts_for(&b.bundle_id.to_lowercase(), false))
            {
                continue;
            }
            // Label with the most frequent app name among the copies (stable).
            let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
            for b in &bundles {
                *counts.entry(b.name.as_str()).or_default() += 1;
            }
            let first = bundles
                .iter()
                .max_by(|a, b| {
                    counts[a.name.as_str()]
                        .cmp(&counts[b.name.as_str()])
                        .then(b.name.len().cmp(&a.name.len()))
                })
                .unwrap();
            let key = product_token(&first.bundle_id.to_lowercase());
            let key = if key.len() >= MIN_TOKEN_LEN {
                key
            } else {
                normalize(&first.name)
            };
            if key.is_empty() || known.mentions(&key) {
                continue;
            }
            let group = groups
                .entry(key.clone())
                .or_insert_with(|| (first.name.clone(), Vec::new()));
            group.0 = first.name.clone();
            group.1.push((top, LeftoverKind::Support));
            with_copies.insert(key);
        }
    }

    // 3. Name-only folders matching an orphaned product.
    // Only for products confirmed by an app copy: short or generic tokens
    // ("network", "crashreporter") would otherwise hit system folders.
    let keys: Vec<String> = with_copies
        .into_iter()
        .filter(|k| k.len() >= MIN_TOKEN_LEN)
        .collect();
    for (root, kind, name) in snapshot.entries() {
        if !matches!(
            kind,
            LeftoverKind::Support | LeftoverKind::Cache | LeftoverKind::Logs
        ) || entry_id(name).is_some()
        {
            continue;
        }
        let normalized = normalize(name);
        if let Some(key) = keys.iter().find(|k| normalized.starts_with(k.as_str())) {
            groups.get_mut(key).unwrap().1.push((root.join(name), kind));
        }
    }

    let mut report = OrphanReport::default();
    for (key, (label, paths)) in groups {
        let mut paths: Vec<(PathBuf, LeftoverKind)> = paths;
        paths.sort_by(|a, b| a.0.cmp(&b.0));
        paths.dedup_by(|a, b| a.0 == b.0 || a.0.starts_with(&b.0));
        let mut items: Vec<Leftover> = paths
            .par_iter()
            .map(|(path, kind)| Leftover {
                path: path.to_string_lossy().into_owned(),
                kind: *kind,
                confidence: Confidence::Possible,
                size: size_hint(path).unwrap_or_else(|| path_size(path)),
                needs_admin: needs_admin(path),
                launch_item: *kind == LeftoverKind::LaunchItem
                    && path.extension().is_some_and(|e| e == "plist"),
            })
            .collect();
        items.sort_by(|a, b| b.size.cmp(&a.size));
        let size: u64 = items.iter().map(|i| i.size).sum();
        if size == 0 {
            continue; // empty cache folders: not worth a row
        }
        report.total += size;
        report.groups.push(OrphanGroup {
            key,
            label,
            size,
            items,
        });
    }
    report.groups.sort_by(|a, b| b.size.cmp(&a.size));
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_bundle_id_entries() {
        assert_eq!(
            entry_id("com.acme.widget.plist").as_deref(),
            Some("com.acme.widget")
        );
        assert_eq!(
            entry_id("ABCDE12345.com.acme.shared").as_deref(),
            Some("com.acme.shared")
        );
        assert_eq!(
            entry_id("group.com.acme.shared").as_deref(),
            Some("com.acme.shared")
        );
        assert_eq!(
            entry_id("local.emin.codeburn-offline.menubar-helper.binarycookies").as_deref(),
            Some("local.emin.codeburn-offline.menubar-helper")
        );
        assert_eq!(entry_id("Google"), None);
        assert_eq!(entry_id("readme.txt"), None);
        assert_eq!(entry_id("com.acme"), None);
    }

    #[test]
    fn product_tokens() {
        assert_eq!(
            product_token("local.emin.codeburn-offline.menubar-helper"),
            "codeburn"
        );
        assert_eq!(
            product_token("org.agentseal.codeburn-desktop.helper"),
            "codeburn"
        );
        assert_eq!(product_token("com.github.githubclient"), "githubclient");
    }
}
