use folder_size::{ItemKind, Level, Phase, SizeEngine};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};

const MB: usize = 1 << 20;

#[test]
fn inactive_engine_defers_scans_and_pending_changes_until_reactivated() {
    let tmp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(tmp.path()).unwrap();
    write(&root, "a/file", MB);
    let engine = scanned(&root);
    let before = engine.status().total_size;
    engine.set_active(false);
    write(&root, "a/added", 2 * MB);
    engine.note_change(&root.join("a/added"), false);
    assert!(!engine.start_scan(root.clone()));
    assert_eq!(engine.process_pending(), 0);
    assert_eq!(engine.status().total_size, before);
    assert!(engine.status().pending_updates > 0);
    engine.set_active(true);
    assert!(engine.process_pending() > 0);
    assert!(engine.status().total_size > before);
}

fn write(root: &Path, rel: &str, bytes: usize) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, vec![7u8; bytes]).unwrap();
}

fn du_bytes(path: &Path) -> u64 {
    let out = Command::new("/usr/bin/du")
        .arg("-sk")
        .arg(path)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    text.split_whitespace()
        .next()
        .unwrap()
        .parse::<u64>()
        .unwrap()
        * 1024
}

fn wait_ready(engine: &SizeEngine) {
    let start = Instant::now();
    while engine.is_scanning() {
        assert!(start.elapsed() < Duration::from_secs(30), "scan timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Canonical path (macOS temp dirs live behind the /var -> /private/var symlink).
fn fixture() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(tmp.path()).unwrap();
    write(&root, "a/big.bin", 3 * MB);
    write(&root, "a/small.txt", 1000);
    write(&root, "b/c/deep.bin", 2 * MB);
    write(&root, "b/node_modules/x/pkg.js", 60 * MB);
    write(&root, "setup.dmg", 55 * MB);
    fs::create_dir_all(root.join("empty")).unwrap();
    fs::hard_link(root.join("a/big.bin"), root.join("b/hardlink.bin")).unwrap();
    std::os::unix::fs::symlink(root.join("a"), root.join("link-to-a")).unwrap();
    (tmp, root)
}

fn scanned(root: &Path) -> Arc<SizeEngine> {
    let engine = Arc::new(SizeEngine::new(root.to_path_buf()));
    assert!(engine.start_scan(root.to_path_buf()));
    wait_ready(&engine);
    engine
}

#[test]
fn totals_match_du_and_hard_links_count_once() {
    let (_tmp, root) = fixture();
    let engine = scanned(&root);
    let status = engine.status();
    assert_eq!(status.phase, Phase::Ready);
    let du = du_bytes(&root);
    assert!(
        status.total_size.abs_diff(du) <= 64 * 1024,
        "ours {} du {du}",
        status.total_size
    );
    // big.bin, small.txt, deep.bin, pkg.js, setup.dmg, hardlink.bin, link-to-a
    assert_eq!(status.total_files, 7);

    // Whichever path the parallel scan reaches first carries the bytes; the other counts 0.
    let listed = |dir: &str, name: &str| {
        engine
            .listing(&root.join(dir), false)
            .unwrap()
            .items
            .iter()
            .any(|i| i.name == name)
    };
    let linked = [listed("a", "big.bin"), listed("b", "hardlink.bin")];
    assert_eq!(
        linked.iter().filter(|x| **x).count(),
        1,
        "hard link counted once: {linked:?}"
    );
}

#[test]
fn listing_sorted_with_small_files_bucket_and_no_symlink_descent() {
    let (_tmp, root) = fixture();
    let engine = scanned(&root);
    let listing = engine.listing(&root, true).unwrap();
    let sizes: Vec<u64> = listing.items.iter().map(|i| i.size).collect();
    assert!(
        sizes.windows(2).all(|w| w[0] >= w[1]),
        "items sorted by size"
    );
    assert!(
        !listing
            .items
            .iter()
            .any(|i| i.name == "link-to-a" && i.kind == ItemKind::Dir)
    );
    let b = listing.items.iter().find(|i| i.name == "b").unwrap();
    assert!(
        b.children
            .as_ref()
            .is_some_and(|c| c.iter().any(|i| i.name == "node_modules"))
    );

    let c = engine.listing(&root.join("b/c"), false).unwrap();
    assert!(
        c.items
            .iter()
            .any(|i| i.name == "deep.bin" && i.kind == ItemKind::File)
    );
    // small.txt (and big.bin when its hard link carried the bytes) land in the bucket.
    let a = engine.listing(&root.join("a"), false).unwrap();
    let small = a.items.iter().find(|i| i.kind == ItemKind::Small).unwrap();
    assert!(small.file_count >= 1);
}

#[test]
fn suggestions_flag_node_modules_and_installers() {
    let (_tmp, root) = fixture();
    let engine = scanned(&root);
    let list = engine.suggestions();
    assert!(
        list.iter()
            .any(|s| s.rule == "nodeModules" && s.level == Level::Safe)
    );
    assert!(
        list.iter()
            .any(|s| s.rule == "installer" && s.name == "setup.dmg")
    );
    assert!(
        list.iter()
            .all(|s| s.size >= folder_size::suggest::MIN_SUGGESTION_SIZE)
    );
}

#[test]
fn incremental_updates_follow_file_system_changes() {
    let (_tmp, root) = fixture();
    let engine = scanned(&root);
    let before = engine.status().total_size;

    // New file in an existing directory.
    write(&root, "a/new.bin", 4 * MB);
    engine.note_change(&root.join("a/new.bin"), false);
    assert_eq!(engine.process_pending(), 1);
    let a = engine.listing(&root.join("a"), false).unwrap();
    assert!(a.items.iter().any(|i| i.name == "new.bin"));
    assert!(engine.status().total_size >= before + 4 * MB as u64);

    // New nested directory: the event names a path the tree has never seen.
    write(&root, "x/y/z/deep.bin", 2 * MB);
    engine.note_change(&root.join("x/y/z/deep.bin"), false);
    engine.process_pending();
    assert!(engine.listing(&root.join("x/y/z"), false).is_some());

    // Removed directory.
    fs::remove_dir_all(root.join("b")).unwrap();
    engine.note_change(&root.join("b"), false);
    engine.process_pending();
    assert!(engine.listing(&root.join("b"), false).is_none());

    let du = du_bytes(&root);
    let ours = engine.status().total_size;
    assert!(
        ours.abs_diff(du) <= 64 * 1024,
        "after updates ours {ours} du {du}"
    );
}

#[test]
fn deep_change_rescans_subtree_and_forget_removes_items() {
    let (_tmp, root) = fixture();
    let engine = scanned(&root);
    write(&root, "b/c/d/e.bin", 5 * MB);
    engine.note_change(&root.join("b"), true);
    engine.process_pending();
    assert!(engine.listing(&root.join("b/c/d"), false).is_some());

    let size = engine.status().total_size;
    engine.forget(&root.join("setup.dmg"));
    assert!(engine.status().total_size <= size - 55 * MB as u64);
}

#[test]
fn snapshot_round_trip() {
    let (_tmp, root) = fixture();
    let engine = scanned(&root);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("sizes.db");
    assert!(engine.save_if_changed(&file).unwrap());
    assert!(
        !engine.save_if_changed(&file).unwrap(),
        "unchanged tree is not rewritten"
    );

    let restored = SizeEngine::new(root.clone());
    assert!(restored.load(&file).unwrap());
    let (a, b) = (engine.status(), restored.status());
    assert_eq!((a.total_size, a.total_files), (b.total_size, b.total_files));
    assert!(b.from_snapshot);
    let names = |e: &SizeEngine| {
        e.listing(&root, false)
            .unwrap()
            .items
            .iter()
            .map(|i| i.name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&engine), names(&restored));
}

#[test]
fn subtree_scan_keeps_rest_of_tree() {
    let (_tmp, root) = fixture();
    let engine = scanned(&root);
    write(&root, "a/more.bin", 2 * MB);
    assert!(engine.start_scan(root.join("a")));
    wait_ready(&engine);
    assert_eq!(
        engine.status().root_path.as_deref(),
        Some(root.to_str().unwrap())
    );
    assert!(engine.listing(&root.join("b"), false).is_some());
    assert!(
        engine
            .listing(&root.join("a"), false)
            .unwrap()
            .items
            .iter()
            .any(|i| i.name == "more.bin")
    );
}

#[test]
fn unreadable_directories_are_counted() {
    let (_tmp, root) = fixture();
    fs::create_dir_all(root.join("locked/inner")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(root.join("locked"), fs::Permissions::from_mode(0o000)).unwrap();
    let engine = scanned(&root);
    fs::set_permissions(root.join("locked"), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(engine.status().unreadable_count >= 1);
}

#[test]
fn trash_size_reported_when_home_trash_is_in_tree() {
    let (_tmp, root) = fixture();
    write(&root, ".Trash/old.bin", 3 * MB);
    // Treat the fixture root as the home folder.
    let engine = scanned(&root);
    let trash = engine.status().trash_size.expect("trash inside tree");
    assert!(trash >= 3 * MB as u64);
}
