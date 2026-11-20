//! `cargo run --release -p folder-size --example scan -- [path]`
use folder_size::SizeEngine;
use std::{path::PathBuf, sync::Arc, time::Instant};

fn main() {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()));
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| home.clone());
    let engine = Arc::new(SizeEngine::new(home));
    let start = Instant::now();
    engine.start_scan(path.clone());
    while engine.is_scanning() {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let s = engine.status();
    println!(
        "{}: {} bytes, {} files, {} unreadable, {:.2}s",
        path.display(),
        s.total_size,
        s.total_files,
        s.unreadable_count,
        start.elapsed().as_secs_f64()
    );
    for item in engine.listing(&path, false).unwrap().items.iter().take(10) {
        println!("  {:>14}  {:?} {}", item.size, item.kind, item.name);
    }
    let sugg = engine.suggestions();
    println!(
        "suggestions: {} totaling {} bytes",
        sugg.len(),
        sugg.iter().map(|s| s.size).sum::<u64>()
    );
    let mut by_rule: std::collections::BTreeMap<&str, (usize, u64)> = Default::default();
    for s in &sugg {
        let e = by_rule.entry(s.rule).or_default();
        e.0 += 1;
        e.1 += s.size;
    }
    for (rule, (n, size)) in by_rule {
        println!("  {rule:<22} {n:>3} items {:>8.2} GB", size as f64 / 1e9);
    }
}
