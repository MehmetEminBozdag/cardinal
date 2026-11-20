use folder_size::{
    scan::{ScanContext, scan_tree},
    suggest::compute_with_min,
};
use std::{
    fs,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

fn write(root: &Path, rel: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, vec![1u8; 5000]).unwrap();
}

#[test]
fn new_rules_match_only_their_targets() {
    let tmp = tempfile::tempdir().unwrap();
    let home = fs::canonicalize(tmp.path()).unwrap();
    let ext = ".vscode/extensions";
    for f in [
        "anthropic.claude-code-2.1.322-darwin-arm64",
        "anthropic.claude-code-2.1.327-darwin-arm64",
        "anthropic.claude-code-2.1.328-darwin-arm64",
        "openai.chatgpt-26.1106.51428-darwin-arm64",
        "openai.chatgpt-26.1111.21308-darwin-arm64",
        "solo.only-1.0.0",
    ] {
        write(&home, &format!("{ext}/{f}/package.json"));
    }
    fs::write(
        home.join(ext).join(".obsolete"),
        r#"{"anthropic.claude-code-2.1.322-darwin-arm64":true,"gone.already-1.0.0":true}"#,
    )
    .unwrap();
    // No .obsolete file: version comparison alone.
    write(&home, ".antigravity/extensions/foo.bar-1.0.0/x");
    write(&home, ".antigravity/extensions/foo.bar-1.2.0/x");

    let chrome = "Library/Application Support/Google/Chrome";
    write(
        &home,
        &format!("{chrome}/Default/Service Worker/CacheStorage/a"),
    );
    write(
        &home,
        &format!("{chrome}/Profile 1/Service Worker/CacheStorage/a"),
    );
    write(
        &home,
        &format!("{chrome}/Default/Service Worker/ScriptCache/a"),
    );
    write(&home, &format!("{chrome}/Default/History"));
    write(&home, "Library/pnpm/store/v10/files/a");
    write(
        &home,
        "Library/Application Support/Google/GoogleUpdater/crx_cache/a",
    );
    write(&home, "Library/Application Support/Code/CachedData/a");
    write(&home, "Library/Application Support/Cursor/GPUCache/a");
    write(&home, "Library/Application Support/Code/User/settings.json");
    write(
        &home,
        "Library/Application Support/Code/WebStorage/4/CacheStorage/a",
    );
    write(
        &home,
        "Library/Application Support/Code/WebStorage/4/IndexedDB/a",
    );
    write(
        &home,
        "Library/Application Support/Code/CachedExtensionVSIXs/x.vsix",
    );
    write(
        &home,
        "Library/Application Support/Antigravity IDE/CachedData/a",
    );
    write(
        &home,
        ".cache/codex-runtimes/codex-runtime-install-GaTnDD/a",
    );
    write(&home, ".cache/codex-runtimes/node-v22/bin");
    write(&home, ".cache/codeburn/daily-cache.v2.json");

    let ctx = ScanContext::new(&home, Arc::new(AtomicBool::new(false)));
    let tree = scan_tree(&home, &ctx).unwrap();
    let found = compute_with_min(&tree, &home, &home, 1);
    let rel = |rule: &str| -> Vec<String> {
        let mut v: Vec<String> = found
            .iter()
            .filter(|s| s.rule == rule)
            .map(|s| {
                s.path
                    .strip_prefix(home.to_str().unwrap())
                    .unwrap()
                    .trim_start_matches('/')
                    .to_owned()
            })
            .collect();
        v.sort();
        v
    };

    assert_eq!(
        rel("oldExtensions"),
        vec![
            ".antigravity/extensions/foo.bar-1.0.0",
            ".vscode/extensions/anthropic.claude-code-2.1.322-darwin-arm64",
            ".vscode/extensions/anthropic.claude-code-2.1.327-darwin-arm64",
            ".vscode/extensions/openai.chatgpt-26.1106.51428-darwin-arm64",
        ]
    );
    assert_eq!(
        rel("chromeServiceWorker"),
        vec![
            format!("{chrome}/Default/Service Worker/CacheStorage"),
            format!("{chrome}/Profile 1/Service Worker/CacheStorage"),
        ]
    );
    assert_eq!(rel("pnpmStore"), vec!["Library/pnpm/store"]);
    assert_eq!(
        rel("googleUpdaterCache"),
        vec!["Library/Application Support/Google/GoogleUpdater/crx_cache"]
    );
    assert_eq!(
        rel("editorCache"),
        vec![
            "Library/Application Support/Antigravity IDE/CachedData",
            "Library/Application Support/Code/CachedData",
            "Library/Application Support/Cursor/GPUCache",
        ]
    );
    let all: Vec<&str> = found.iter().map(|s| s.path.as_str()).collect();
    for kept in [
        "claude-code-2.1.328",
        "chatgpt-26.1111",
        "solo.only",
        "foo.bar-1.2.0",
        "ScriptCache",
        "History",
        "Code/User",
    ] {
        assert!(
            !all.iter().any(|p| p.contains(kept)),
            "{kept} must stay: {all:?}"
        );
    }
    assert_eq!(
        rel("editorWebCache"),
        vec!["Library/Application Support/Code/WebStorage/4/CacheStorage"]
    );
    assert_eq!(
        rel("vsixCache"),
        vec!["Library/Application Support/Code/CachedExtensionVSIXs"]
    );
    assert_eq!(
        rel("codexRuntimeInstall"),
        vec![".cache/codex-runtimes/codex-runtime-install-GaTnDD"]
    );
    // ~/.cache is offered per tool; codex-runtimes itself is not (only its install leftovers).
    assert_eq!(rel("toolCache"), vec![".cache/codeburn"]);
    for kept in ["IndexedDB", "node-v22"] {
        assert!(
            !all.iter().any(|p| p.contains(kept)),
            "{kept} must stay: {all:?}"
        );
    }
}
