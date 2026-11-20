//! Read-only: `cargo run --release -p folder-size --example apps -- [/Applications/X.app]`
use folder_size::apps::{Locations, analyze, list_apps};
use std::path::{Path, PathBuf};

fn main() {
    let home = PathBuf::from(std::env::var("HOME").unwrap());
    let loc = Locations::system(&home);
    let none = |_: &Path| None;
    match std::env::args().nth(1) {
        Some(app) => {
            let plan = analyze(&loc, Path::new(&app), &[], &none).expect("not a removable app");
            println!(
                "{} {} running={} receipts={:?}",
                plan.app.name, plan.app.bundle_id, plan.running, plan.receipts
            );
            for i in plan.items {
                println!(
                    "  {:>12} {:?} {:?} admin={} {}",
                    i.size, i.confidence, i.kind, i.needs_admin, i.path
                );
            }
        }
        None => {
            let start = std::time::Instant::now();
            let apps = list_apps(&loc, &none);
            println!(
                "{} apps in {:.1}s",
                apps.len(),
                start.elapsed().as_secs_f64()
            );
            for a in apps.iter().take(15) {
                println!(
                    "  {:>12} + {:>12}  {} ({}){}",
                    a.app_size,
                    a.leftover_size,
                    a.app.name,
                    a.app.bundle_id,
                    if a.running { " [running]" } else { "" }
                );
            }
        }
    }
}
