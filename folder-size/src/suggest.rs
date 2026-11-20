//! "Safe to clean" suggestions. Only reports; never deletes anything.

use crate::tree::{Dir, ItemKind};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::OnceLock,
};

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    /// Regenerated automatically by the owning tool or app.
    Safe,
    /// Worth a look; may contain something the user wants to keep.
    Review,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    /// Stable id; the UI maps it to a translated title and explanation.
    pub rule: &'static str,
    pub level: Level,
    pub path: String,
    pub name: String,
    pub kind: ItemKind,
    pub size: u64,
}

enum Match {
    /// Exactly this directory (relative to home).
    Exact(&'static str),
    /// Each entry of this directory is a separate suggestion.
    ChildrenOf(&'static str),
    /// Any directory with this name.
    DirName(&'static str),
    /// Big files with one of these extensions.
    FileExt(&'static [&'static str]),
    /// Path relative to home where `*` matches any one folder name and
    /// `{a,b}` matches one of the listed names.
    Pattern(&'static str),
    /// A VS Code-style extensions folder: superseded extension versions.
    OldExtensions(&'static str),
    /// Path relative to the per-user cache dir (`getconf DARWIN_USER_CACHE_DIR`).
    UserCache(&'static str),
}

struct Rule {
    id: &'static str,
    level: Level,
    matcher: Match,
}

const RULES: &[Rule] = &[
    Rule {
        id: "appCache",
        level: Level::Safe,
        matcher: Match::ChildrenOf("Library/Caches"),
    },
    Rule {
        id: "logs",
        level: Level::Safe,
        matcher: Match::Exact("Library/Logs"),
    },
    Rule {
        id: "xcodeDerivedData",
        level: Level::Safe,
        matcher: Match::Exact("Library/Developer/Xcode/DerivedData"),
    },
    Rule {
        id: "xcodeDeviceSupport",
        level: Level::Safe,
        matcher: Match::Exact("Library/Developer/Xcode/iOS DeviceSupport"),
    },
    Rule {
        id: "simulatorCache",
        level: Level::Safe,
        matcher: Match::Exact("Library/Developer/CoreSimulator/Caches"),
    },
    Rule {
        id: "xcodeArchives",
        level: Level::Review,
        matcher: Match::Exact("Library/Developer/Xcode/Archives"),
    },
    Rule {
        id: "simulatorDevices",
        level: Level::Review,
        matcher: Match::Exact("Library/Developer/CoreSimulator/Devices"),
    },
    Rule {
        id: "androidEmulators",
        level: Level::Review,
        matcher: Match::Exact(".android/avd"),
    },
    Rule {
        id: "androidSystemImages",
        level: Level::Review,
        matcher: Match::Exact("Library/Android/sdk/system-images"),
    },
    Rule {
        id: "packageCache",
        level: Level::Safe,
        matcher: Match::Exact(".gradle/caches"),
    },
    Rule {
        id: "packageCache",
        level: Level::Safe,
        matcher: Match::Exact(".npm/_cacache"),
    },
    Rule {
        id: "packageCache",
        level: Level::Safe,
        matcher: Match::Exact(".cargo/registry"),
    },
    Rule {
        id: "packageCache",
        level: Level::Safe,
        matcher: Match::Exact("go/pkg/mod"),
    },
    Rule {
        id: "toolCache",
        level: Level::Review,
        matcher: Match::ChildrenOf(".cache"),
    },
    Rule {
        id: "spotifyCache",
        level: Level::Safe,
        matcher: Match::Exact("Library/Application Support/Spotify/PersistentCache"),
    },
    Rule {
        id: "deviceBackups",
        level: Level::Review,
        matcher: Match::Exact("Library/Application Support/MobileSync/Backup"),
    },
    Rule {
        id: "docker",
        level: Level::Review,
        matcher: Match::Exact("Library/Containers/com.docker.docker"),
    },
    Rule {
        id: "trash",
        level: Level::Review,
        matcher: Match::Exact(".Trash"),
    },
    Rule {
        id: "downloads",
        level: Level::Review,
        matcher: Match::Exact("Downloads"),
    },
    Rule {
        id: "oldExtensions",
        level: Level::Safe,
        matcher: Match::OldExtensions(".vscode/extensions"),
    },
    Rule {
        id: "oldExtensions",
        level: Level::Safe,
        matcher: Match::OldExtensions(".vscode-insiders/extensions"),
    },
    Rule {
        id: "oldExtensions",
        level: Level::Safe,
        matcher: Match::OldExtensions(".cursor/extensions"),
    },
    Rule {
        id: "oldExtensions",
        level: Level::Safe,
        matcher: Match::OldExtensions(".antigravity/extensions"),
    },
    Rule {
        id: "oldExtensions",
        level: Level::Safe,
        matcher: Match::OldExtensions(".windsurf/extensions"),
    },
    Rule {
        id: "chromeServiceWorker",
        level: Level::Safe,
        matcher: Match::Pattern(
            "Library/Application Support/Google/Chrome/*/Service Worker/CacheStorage",
        ),
    },
    Rule {
        id: "pnpmStore",
        level: Level::Review,
        matcher: Match::Exact("Library/pnpm/store"),
    },
    Rule {
        id: "googleUpdaterCache",
        level: Level::Safe,
        matcher: Match::Exact("Library/Application Support/Google/GoogleUpdater/crx_cache"),
    },
    Rule {
        id: "editorCache",
        level: Level::Safe,
        matcher: Match::Pattern(
            "Library/Application Support/{Code,Code - Insiders,Antigravity,Antigravity IDE,Cursor,Windsurf}/{Cache,CachedData,Code Cache,GPUCache}",
        ),
    },
    Rule {
        id: "editorWebCache",
        level: Level::Safe,
        matcher: Match::Pattern(
            "Library/Application Support/{Code,Code - Insiders,Antigravity,Antigravity IDE,Cursor,Windsurf}/WebStorage/*/CacheStorage",
        ),
    },
    Rule {
        id: "vsixCache",
        level: Level::Safe,
        matcher: Match::Pattern(
            "Library/Application Support/{Code,Code - Insiders,Antigravity,Antigravity IDE,Cursor,Windsurf}/CachedExtensionVSIXs",
        ),
    },
    Rule {
        id: "codexRuntimeInstall",
        level: Level::Safe,
        matcher: Match::Pattern(".cache/codex-runtimes/codex-runtime-install-*"),
    },
    Rule {
        id: "clangModuleCache",
        level: Level::Safe,
        matcher: Match::UserCache("clang/ModuleCache"),
    },
    Rule {
        id: "nodeModules",
        level: Level::Safe,
        matcher: Match::DirName("node_modules"),
    },
    Rule {
        id: "installer",
        level: Level::Review,
        matcher: Match::FileExt(&["dmg", "pkg", "iso", "xip", "ipa"]),
    },
];

/// Suggestions at or above this size are reported.
pub const MIN_SUGGESTION_SIZE: u64 = 50 * 1024 * 1024;

/// One path segment of a [`Match::Pattern`].
enum Segment {
    Any,
    OneOf(Vec<&'static str>),
    /// `name-*`: any folder name starting with the prefix.
    Prefix(&'static str),
}

fn parse_pattern(pattern: &'static str) -> Vec<Segment> {
    pattern
        .split('/')
        .map(|seg| {
            if seg == "*" {
                Segment::Any
            } else if let Some(prefix) = seg.strip_suffix('*') {
                Segment::Prefix(prefix)
            } else if let Some(inner) = seg.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                Segment::OneOf(inner.split(',').collect())
            } else {
                Segment::OneOf(vec![seg])
            }
        })
        .collect()
}

fn segment_matches(part: &std::ffi::OsStr, seg: &Segment) -> bool {
    match seg {
        Segment::Any => true,
        Segment::OneOf(names) => names.iter().any(|n| part == std::ffi::OsStr::new(n)),
        Segment::Prefix(prefix) => part.to_string_lossy().starts_with(prefix),
    }
}

fn pattern_matches(segments: &[Segment], relative: &Path) -> bool {
    let parts: Vec<_> = relative.components().map(|c| c.as_os_str()).collect();
    parts.len() == segments.len()
        && parts
            .iter()
            .zip(segments)
            .all(|(part, seg)| segment_matches(part, seg))
}

/// Whether `relative` is a strict ancestor of something the pattern can match.
fn pattern_below(segments: &[Segment], relative: &Path) -> bool {
    let parts: Vec<_> = relative.components().map(|c| c.as_os_str()).collect();
    parts.len() < segments.len()
        && parts
            .iter()
            .zip(segments)
            .all(|(part, seg)| segment_matches(part, seg))
}

/// `getconf DARWIN_USER_CACHE_DIR`, resolved through `/var -> /private/var`.
fn user_cache_dir() -> Option<&'static Path> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| {
        let out = std::process::Command::new("/usr/bin/getconf")
            .arg("DARWIN_USER_CACHE_DIR")
            .output()
            .ok()?;
        let path = PathBuf::from(String::from_utf8(out.stdout).ok()?.trim());
        std::fs::canonicalize(&path).ok().or(Some(path))
    })
    .as_deref()
}

/// Splits `publisher.name-1.2.3-darwin-arm64` into the id and version numbers.
pub fn extension_version(folder: &str) -> Option<(&str, Vec<u64>)> {
    let bytes = folder.as_bytes();
    let dash = (1..bytes.len()).find(|&i| bytes[i - 1] == b'-' && bytes[i].is_ascii_digit())?;
    let id = &folder[..dash - 1];
    let version = folder[dash..].split('-').next()?;
    let numbers = version
        .split('.')
        .map(|p| {
            p.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .unwrap_or(0)
        })
        .collect();
    Some((id, numbers))
}

/// Extension folders superseded by a newer version of the same extension, or
/// listed in the editor's own `.obsolete` file.
pub fn obsolete_extensions(dir: &Path, folders: &[&str]) -> HashSet<String> {
    let mut obsolete: HashSet<String> = std::fs::read(dir.join(".obsolete"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<HashMap<String, bool>>(&bytes).ok())
        .map(|map| {
            map.into_iter()
                .filter(|(_, v)| *v)
                .map(|(k, _)| k)
                .collect()
        })
        .unwrap_or_default();
    obsolete.retain(|name| folders.contains(&name.as_str()));
    let mut newest: HashMap<&str, Vec<u64>> = HashMap::new();
    for folder in folders {
        if let Some((id, version)) = extension_version(folder) {
            let entry = newest.entry(id).or_default();
            if version > *entry {
                *entry = version;
            }
        }
    }
    for folder in folders {
        if let Some((id, version)) = extension_version(folder)
            && newest.get(id).is_some_and(|max| version < *max)
        {
            obsolete.insert((*folder).to_owned());
        }
    }
    obsolete
}

struct Rules<'a> {
    exact: HashMap<PathBuf, &'a Rule>,
    children_of: HashMap<PathBuf, &'a Rule>,
    names: HashMap<&'static str, &'a Rule>,
    exts: HashMap<&'static str, &'a Rule>,
    patterns: Vec<(Vec<Segment>, &'a Rule)>,
    old_extensions: HashMap<PathBuf, &'a Rule>,
    home: &'a Path,
    min_size: u64,
}

pub fn compute(root: &Dir, root_path: &Path, home: &Path) -> Vec<Suggestion> {
    compute_with_min(root, root_path, home, MIN_SUGGESTION_SIZE)
}

/// [`compute`] with a custom size threshold (tests use tiny files).
pub fn compute_with_min(
    root: &Dir,
    root_path: &Path,
    home: &Path,
    min_size: u64,
) -> Vec<Suggestion> {
    let mut rules = Rules {
        exact: HashMap::new(),
        children_of: HashMap::new(),
        names: HashMap::new(),
        exts: HashMap::new(),
        patterns: Vec::new(),
        old_extensions: HashMap::new(),
        home,
        min_size,
    };
    for rule in RULES {
        match rule.matcher {
            Match::Exact(rel) => {
                rules.exact.insert(home.join(rel), rule);
            }
            Match::ChildrenOf(rel) => {
                rules.children_of.insert(home.join(rel), rule);
            }
            Match::DirName(name) => {
                rules.names.insert(name, rule);
            }
            Match::FileExt(list) => list.iter().for_each(|e| {
                rules.exts.insert(e, rule);
            }),
            Match::Pattern(pattern) => rules.patterns.push((parse_pattern(pattern), rule)),
            Match::OldExtensions(rel) => {
                rules.old_extensions.insert(home.join(rel), rule);
            }
            Match::UserCache(rel) => {
                if let Some(base) = user_cache_dir() {
                    rules.exact.insert(base.join(rel), rule);
                }
            }
        }
    }

    let mut out = Vec::new();
    walk(root, root_path, true, &rules, &mut out);
    out.sort_by(|a, b| a.level.cmp(&b.level).then(b.size.cmp(&a.size)));
    out
}

fn push(
    out: &mut Vec<Suggestion>,
    rules: &Rules,
    rule: &Rule,
    path: &Path,
    name: &str,
    kind: ItemKind,
    size: u64,
) {
    if size >= rules.min_size {
        out.push(Suggestion {
            rule: rule.id,
            level: rule.level,
            path: path.to_string_lossy().into_owned(),
            name: name.to_owned(),
            kind,
            size,
        });
    }
}

fn walk(dir: &Dir, path: &Path, is_root: bool, rules: &Rules, out: &mut Vec<Suggestion>) {
    if dir.size < rules.min_size {
        return;
    }
    let by_name = (!is_root).then(|| rules.names.get(&*dir.name)).flatten();
    let by_pattern = path.strip_prefix(rules.home).ok().and_then(|rel| {
        rules
            .patterns
            .iter()
            .find(|(segs, _)| pattern_matches(segs, rel))
            .map(|(_, r)| r)
    });
    if let Some(rule) = rules.exact.get(path).or(by_name).or(by_pattern) {
        push(out, rules, rule, path, &dir.name, ItemKind::Dir, dir.size);
        return;
    }
    if let Some(rule) = rules.children_of.get(path) {
        for child in &dir.children {
            let child_path = path.join(&*child.name);
            // A more specific pattern lives below this child: look inside instead.
            let more_specific = child_path.strip_prefix(rules.home).is_ok_and(|rel| {
                rules
                    .patterns
                    .iter()
                    .any(|(segs, _)| pattern_below(segs, rel))
            });
            if more_specific {
                walk(child, &child_path, false, rules, out);
                continue;
            }
            push(
                out,
                rules,
                rule,
                &path.join(&*child.name),
                &child.name,
                ItemKind::Dir,
                child.size,
            );
        }
        for file in &dir.files {
            push(
                out,
                rules,
                rule,
                &path.join(&*file.name),
                &file.name,
                ItemKind::File,
                file.size,
            );
        }
        return;
    }
    if let Some(rule) = rules.old_extensions.get(path) {
        let folders: Vec<&str> = dir.children.iter().map(|c| &*c.name).collect();
        let obsolete = obsolete_extensions(path, &folders);
        for child in dir.children.iter().filter(|c| obsolete.contains(&*c.name)) {
            push(
                out,
                rules,
                rule,
                &path.join(&*child.name),
                &child.name,
                ItemKind::Dir,
                child.size,
            );
        }
        return;
    }
    for file in &dir.files {
        let ext = Path::new(&*file.name)
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase());
        if let Some(rule) = ext.as_deref().and_then(|e| rules.exts.get(e)) {
            push(
                out,
                rules,
                rule,
                &path.join(&*file.name),
                &file.name,
                ItemKind::File,
                file.size,
            );
        }
    }
    for child in &dir.children {
        walk(child, &path.join(&*child.name), false, rules, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_extension_versions() {
        assert_eq!(
            extension_version("anthropic.claude-code-2.1.322-darwin-arm64"),
            Some(("anthropic.claude-code", vec![2, 1, 322]))
        );
        assert_eq!(
            extension_version("openai.chatgpt-26.1106.51428-darwin-arm64"),
            Some(("openai.chatgpt", vec![26, 1106, 51428]))
        );
        assert_eq!(
            extension_version("42crunch.vscode-openapi-4.25.1"),
            Some(("42crunch.vscode-openapi", vec![4, 25, 1]))
        );
        assert_eq!(extension_version("extensions.json"), None);
    }

    #[test]
    fn patterns_match_wildcards_and_alternatives() {
        let segs = parse_pattern(
            "Library/Application Support/Google/Chrome/*/Service Worker/CacheStorage",
        );
        assert!(pattern_matches(
            &segs,
            Path::new(
                "Library/Application Support/Google/Chrome/Profile 2/Service Worker/CacheStorage"
            )
        ));
        assert!(!pattern_matches(
            &segs,
            Path::new("Library/Application Support/Google/Chrome/Default/Service Worker")
        ));
        let segs = parse_pattern("Library/Application Support/{Code,Cursor}/{Cache,GPUCache}");
        assert!(pattern_matches(
            &segs,
            Path::new("Library/Application Support/Cursor/GPUCache")
        ));
        assert!(!pattern_matches(
            &segs,
            Path::new("Library/Application Support/Code/User")
        ));
    }

    #[test]
    fn prefix_segments_and_ancestors() {
        let segs = parse_pattern(".cache/codex-runtimes/codex-runtime-install-*");
        assert!(pattern_matches(
            &segs,
            Path::new(".cache/codex-runtimes/codex-runtime-install-GaTnDD")
        ));
        assert!(!pattern_matches(
            &segs,
            Path::new(".cache/codex-runtimes/node-v22")
        ));
        assert!(pattern_below(&segs, Path::new(".cache/codex-runtimes")));
        assert!(!pattern_below(&segs, Path::new(".cache/codeburn")));
    }
}
