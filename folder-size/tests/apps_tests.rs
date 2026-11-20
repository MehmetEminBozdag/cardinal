use folder_size::apps::{Confidence, LeftoverKind, Locations, analyze, list_apps};
use std::{fs, path::Path};

fn write(path: &Path, bytes: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, vec![1u8; bytes]).unwrap();
}

fn fake_app(dir: &Path, name: &str, id: &str) {
    let app = dir.join(format!("{name}.app"));
    write(&app.join("Contents/MacOS").join(name), 2 << 20);
    fs::write(
        app.join("Contents/Info.plist"),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>{id}</string>
<key>CFBundleName</key><string>{name}</string>
<key>CFBundleShortVersionString</key><string>1.2</string>
</dict></plist>"#
        ),
    )
    .unwrap();
}

#[test]
fn finds_leftovers_in_user_and_system_library() {
    let tmp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(tmp.path()).unwrap();
    let apps = root.join("Applications");
    let lib = root.join("home/Library");
    let sys = root.join("SysLibrary");
    fake_app(&apps, "Widget", "com.acme.widget");
    fake_app(&apps, "Gadget", "com.other.gadget");
    fake_app(&apps, "AppleThing", "com.apple.thing");

    write(&lib.join("Application Support/Widget/data.db"), 3 << 20);
    write(&lib.join("Caches/com.acme.widget/cache.bin"), 1 << 20);
    write(&lib.join("Preferences/com.acme.widget.plist"), 100);
    write(&lib.join("Containers/com.acme.widget.Share/x"), 100);
    write(
        &lib.join("Saved Application State/com.acme.widget.savedState/s"),
        100,
    );
    write(&lib.join("LaunchAgents/com.acme.updater.plist"), 100);
    let recent = lib.join("Application Support/com.apple.sharedfilelist/com.apple.LSSharedFileList.ApplicationRecentDocuments");
    write(&recent.join("com.acme.widget.sfl3"), 100);
    write(&sys.join("LaunchDaemons/com.acme.helper.plist"), 100);
    write(&sys.join("Application Support/Widget/shared.bin"), 1 << 20);
    // Belongs to another app and must not be matched.
    write(&lib.join("Application Support/Gadget/keep.bin"), 1 << 20);
    write(&lib.join("Caches/com.acme.widgetpro/keep"), 100);

    let locations = Locations {
        app_dirs: vec![apps.clone()],
        user_library: lib.clone(),
        system_library: sys.clone(),
        user_temp_dirs: vec![],
    };
    let none = |_: &Path| None;

    let listed = list_apps(&locations, &none);
    let names: Vec<&str> = listed.iter().map(|a| a.app.name.as_str()).collect();
    assert!(names.contains(&"Widget") && names.contains(&"Gadget"));
    assert!(
        !names.contains(&"AppleThing"),
        "Apple apps are never offered"
    );
    let widget = listed.iter().find(|a| a.app.name == "Widget").unwrap();
    assert!(widget.leftover_size >= 5 << 20);

    let extra = vec![root.join("home/Downloads/com.acme.widget-crash.log")];
    write(&extra[0], 100);
    let plan = analyze(&locations, &apps.join("Widget.app"), &extra, &none).unwrap();
    let has = |rel: &Path, conf: Confidence| {
        plan.items
            .iter()
            .any(|i| Path::new(&i.path) == rel && i.confidence == conf)
    };
    assert!(has(&apps.join("Widget.app"), Confidence::Certain));
    assert!(has(
        &lib.join("Application Support/Widget"),
        Confidence::Likely
    ));
    assert!(has(
        &lib.join("Caches/com.acme.widget"),
        Confidence::Certain
    ));
    assert!(has(
        &lib.join("Preferences/com.acme.widget.plist"),
        Confidence::Certain
    ));
    assert!(has(
        &lib.join("Containers/com.acme.widget.Share"),
        Confidence::Certain
    ));
    assert!(has(
        &lib.join("Saved Application State/com.acme.widget.savedState"),
        Confidence::Certain
    ));
    assert!(has(
        &lib.join("LaunchAgents/com.acme.updater.plist"),
        Confidence::Likely
    ));
    assert!(has(
        &sys.join("LaunchDaemons/com.acme.helper.plist"),
        Confidence::Likely
    ));
    assert!(has(
        &sys.join("Application Support/Widget"),
        Confidence::Likely
    ));
    assert!(has(&extra[0], Confidence::Possible));
    assert!(
        plan.items
            .iter()
            .any(|i| i.kind == LeftoverKind::RecentDocuments
                && i.confidence == Confidence::Certain
                && i.path.ends_with("com.acme.widget.sfl3"))
    );
    assert!(
        plan.items
            .iter()
            .any(|i| i.launch_item && i.kind == LeftoverKind::LaunchItem)
    );
    // Another product of the same vendor: listed, but never preselected.
    assert!(has(
        &lib.join("Caches/com.acme.widgetpro"),
        Confidence::Possible
    ));
    let paths: Vec<&str> = plan.items.iter().map(|i| i.path.as_str()).collect();
    assert!(!paths.iter().any(|p| p.contains("Gadget")), "{paths:?}");
    assert!(!plan.running);
}
