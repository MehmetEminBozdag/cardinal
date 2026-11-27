# folder-size

Folder size analysis behind Cardinal's **Sizes** tab.

- **Scanner** (`scan.rs`): parallel (rayon) walk using `getattrlistbulk(2)`, falling back to
  `readdir` + `lstat`. Measures *allocated* size, the same number `du` reports. Hard links are
  counted once, symlinks are not followed, and other volumes are skipped. Home folder on an
  M-series Mac: ~118 GB / 850k files in ~7 s, within a few MB of `du -sk`.
- **Tree** (`tree.rs`): per-directory totals; files ≥ 1 MiB are kept individually, smaller
  ones are counted in a "small files" bucket per folder to keep memory flat.
- **Engine** (`engine.rs`): owns the tree. `note_change(path, deep)` records an FSEvent;
  `process_pending()` re-reads only the affected directories (new subfolders are scanned,
  vanished ones dropped, deep events rescan the subtree) and updates totals up to the root.
  Cardinal's existing FSEvents stream drives it. Disk scans run only while the Sizes tab
  is selected and Cardinal is foreground, with a 15-second grace after focus loss.
  Switching tabs suspends immediately. Suspension cancels full/incremental work;
  changes are retained (and coalesced if the queue grows) for the next activation.
- **Suggestions** (`suggest.rs`): well-known caches and temporary folders (app caches, Xcode
  DerivedData, package-manager caches, `node_modules`, Android emulators, installers, ...)
  above 50 MB, labelled `safe` or `review`. The crate never deletes anything.
- **Snapshot** (`persist.rs`): postcard + zstd file next to `cardinal.db`
  (`folder-sizes.db`), shown on the next launch. A snapshot older than 10 minutes is
  refreshed only when the Sizes tab is eligible to scan, never merely on startup.

Tauri glue lives in `cardinal/src-tauri/src/folder_sizes.rs`
(`get_size_status`, `start_size_scan`, `cancel_size_scan`, `get_size_listing`,
`get_size_suggestions`); the UI is `SizesPanel.tsx`, `SizeTreemap.tsx` and
`useFolderSizes.ts`. Move to Trash goes through the existing confirmed `move_to_trash` command,
which also removes the item from the size tree right away.

```bash
cargo test -p folder-size
cargo run --release -p folder-size --example scan -- ~
```

Known limits: a hard link counted under one folder may be counted again after an incremental
refresh of a different folder; names that are not valid UTF-8 are shown lossily.
