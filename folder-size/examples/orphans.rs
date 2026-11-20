//! Read-only: `cargo run --release -p folder-size --example orphans`
use folder_size::{
    apps::Locations,
    orphans::{find_orphans, spotlight_app_ids},
};
use std::path::{Path, PathBuf};

fn main() {
    let home = PathBuf::from(std::env::var("HOME").unwrap());
    let loc = Locations::system(&home);
    let skip = [
        home.join("Library/Application Support"),
        home.join(".Trash"),
    ];
    let ids = spotlight_app_ids(&skip);
    let start = std::time::Instant::now();
    let report = find_orphans(&loc, &ids, &|_: &Path| None);
    println!(
        "{} groups, {} bytes, {:.1}s ({} spotlight ids)",
        report.groups.len(),
        report.total,
        start.elapsed().as_secs_f64(),
        ids.len()
    );
    for g in report.groups.iter().take(25) {
        println!("{:>12}  {} [{}]", g.size, g.label, g.key);
        for i in g.items.iter().take(6) {
            println!(
                "      {:>12} {:?} admin={} {}",
                i.size,
                i.kind,
                i.needs_admin,
                i.path.replace(home.to_str().unwrap(), "~")
            );
        }
    }
}
