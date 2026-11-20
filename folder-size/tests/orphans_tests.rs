use folder_size::{apps::Locations, orphans::find_orphans};
use std::{fs, path::Path};

fn write(path: &Path, bytes: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, vec![1u8; bytes]).unwrap();
}

fn bundle(dir: &Path, name: &str, id: &str) {
    write(&dir.join("Contents/MacOS/bin"), 1 << 20);
    fs::write(
        dir.join("Contents/Info.plist"),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>{id}</string>
<key>CFBundleName</key><string>{name}</string>
</dict></plist>"#
        ),
    )
    .unwrap();
}

#[test]
fn groups_leftovers_of_uninstalled_apps() {
    let tmp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(tmp.path()).unwrap();
    let apps = root.join("Applications");
    let lib = root.join("home/Library");
    let sys = root.join("SysLibrary");

    // Installed: Token Counter (vendor local.emin) and Widget (vendor com.acme).
    bundle(
        &apps.join("Token Counter.app"),
        "Token Counter",
        "local.emin.tokencounter",
    );
    bundle(&apps.join("Widget.app"), "Widget", "com.acme.widget");
    write(&lib.join("Application Support/token-counter/db"), 1000);
    write(&lib.join("Preferences/com.acme.updater.plist"), 10); // installed vendor
    write(&lib.join("Preferences/com.apple.finder.plist"), 10); // Apple

    // Gone: CodeBurn (two vendors) with backups, prefs, name-only folders.
    let backups = lib.join("Application Support/CodeBurnOffline-RecoveryBackups");
    bundle(
        &backups.join("2026-1/Applications/CodeBurn Offline.app.backup"),
        "CodeBurn Offline",
        "org.agentseal.codeburn-desktop",
    );
    bundle(
        &backups.join("2026-2/Applications/CodeBurn Offline.app.backup"),
        "CodeBurn Offline",
        "org.agentseal.codeburn-desktop",
    );
    // Same personal namespace as the installed Token Counter: still an orphan.
    bundle(
        &backups.join("2026-3/CodeBurn Offline Desktop.app.backup"),
        "CodeBurn Offline Desktop",
        "local.emin.codeburn-offline-desktop",
    );
    write(
        &lib.join("Preferences/local.emin.codeburn-offline.menubar-helper.plist"),
        10,
    );
    write(
        &lib.join("Application Support/codeburn-desktop/state"),
        5000,
    );
    write(
        &lib.join("HTTPStorages/org.agentseal.codeburn-menubar/x"),
        10,
    );
    write(
        &lib.join("Preferences/org.agentseal.codeburn-menubar.plist"),
        10,
    );
    // System components and name-only folders without an app copy.
    write(&lib.join("Preferences/org.cups.PrintingPrefs.plist"), 10);
    write(&lib.join("Caches/io.branch.network.cache/c"), 10);
    write(&lib.join("Application Support/networkserviceproxy/p"), 10);
    // Gone: an unrelated app.
    write(&lib.join("Caches/com.gone.oldeditor/c"), 3000);
    write(
        &sys.join("LaunchDaemons/com.gone.oldeditor.helper.plist"),
        10,
    );

    let loc = Locations {
        app_dirs: vec![apps],
        user_library: lib.clone(),
        system_library: sys.clone(),
        user_temp_dirs: vec![],
    };
    let report = find_orphans(&loc, &[], &|_| None);
    let group = |key: &str| report.groups.iter().find(|g| g.key == key);

    let codeburn = group("codeburn").expect("codeburn group");
    assert_eq!(codeburn.label, "CodeBurn Offline");
    let paths: Vec<&str> = codeburn.items.iter().map(|i| i.path.as_str()).collect();
    for expected in [
        "Application Support/CodeBurnOffline-RecoveryBackups",
        "Application Support/codeburn-desktop",
        "HTTPStorages/org.agentseal.codeburn-menubar",
        "Preferences/org.agentseal.codeburn-menubar.plist",
        "Preferences/local.emin.codeburn-offline.menubar-helper.plist",
    ] {
        assert!(
            paths.iter().any(|p| p.ends_with(expected)),
            "missing {expected}: {paths:?}"
        );
    }
    assert!(codeburn.size >= 2 << 20);

    let oldeditor = group("oldeditor").expect("oldeditor group");
    assert!(oldeditor.items.iter().any(|i| i.launch_item));

    let all: Vec<&str> = report
        .groups
        .iter()
        .flat_map(|g| g.items.iter().map(|i| i.path.as_str()))
        .collect();
    for kept in [
        "token-counter",
        "com.acme.updater",
        "com.apple.finder",
        "org.cups",
        "networkserviceproxy",
    ] {
        assert!(
            !all.iter().any(|p| p.contains(kept)),
            "{kept} must not be an orphan: {all:?}"
        );
    }
    assert!(
        report
            .groups
            .iter()
            .flat_map(|g| &g.items)
            .all(|i| i.confidence == folder_size::apps::Confidence::Possible)
    );
    assert_eq!(
        report.total,
        report.groups.iter().map(|g| g.size).sum::<u64>()
    );
}
