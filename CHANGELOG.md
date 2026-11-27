# Changelog

## v0.1.24-fork.2 — 2026-11-27

- Incorporate EminSet's folder-size and app-uninstall work through `a4658a1`: list/treemap views, disk and Trash usage, cleanup suggestions, app leftovers, and persistent Move to Trash history.
- Suspend full and incremental size scans immediately when leaving Sizes; allow a single 15-second grace after Cardinal loses focus with Sizes selected. Repeated background notifications do not extend the deadline.
- Check cancellation during file enumeration, retain/coalesce pending changes, and defer automatic snapshot refresh until the view is eligible. Ordinary filename indexing remains independent.
- Update English/Turkish fork documentation and integration notes. The three `contrib/*` branches remain for existing PRs; use `public/cardinal-enhancements` for the complete source.
- Package identity: the native app still reports `0.1.24`; `v0.1.24-fork.2` identifies this fork snapshot. The Apple Silicon DMG is the previously built and installed package, unchanged by subsequent test/documentation commits, with an ad-hoc signature and no Apple notarization.

Verification recorded during implementation: 66 native tests, 27 folder-size tests, two folder-size hook tests, TypeScript, Clippy, formatting, and the DMG build passed. The wider frontend run recorded four unhandled context-menu mock errors and is not a clean full-suite pass. No controlled full-index CPU before/after benchmark or real-file deletion test is claimed. Release publication additionally checks the DMG checksum and installed-binary match.

## Fork follow-up — 2026-11-01

- Probe the visible, non-minimized WebView every 15 seconds; reload after 60 seconds without a DOM/IPC response, with at most three attempts per ten minutes.
- Record renderer-health events in a bounded local log and tolerate a dropped search-response receiver during renderer recovery.
- A packaged killed-WebContent recovery test passed on 2026-11-01. The original high-CPU trigger was not established; this is renderer recovery, not a GPU-driver or general CPU fix.

## Fork follow-up — 2026-09-07

- Defer the Trash confirmation until after the native context-menu callback returns. Unit tests and packaging passed; the final packaged multi-delete rerun remained pending at that snapshot.

Fork handoff and upstream status: [integration guide](docs/FORK-INTEGRATION.tr-TR.md), checked 2026-11-20. Earlier 0.1.24 notes mention extension filters; the shipped visual filter component currently exposes type, modified date, and size only. Extension syntax remains available in the query language.

## 0.1.24 — 2026-08-28
- Move advanced search controls into the interface, including newest-first sorting, folders as a type, date/size/extension filters, and one-click inclusion of excluded paths.
- Add a hideable workspace toolbar and expanded Search, View, and Index menus for saved searches, favorites, coverage, operations, and duplicate analysis.
- Add on-demand duplicate analysis for the current results, distinguishing APFS clones, hard links, and physical copies when macOS reports reliable clone mapping.
- Improve large-result ordering, background CPU control, Cmd+Q shutdown responsiveness, and direct Move to Trash workflows.
- Localize the added features and publish release notes for all 15 supported locales.

## 0.1.23 — 2026-03-25
- Reduce power consumption by expanding the default ignored paths to cover more macOS cache, log, metadata, and runtime directories.
- Further reduce background work by making the filesystem event watcher honor ignored paths.
- Improve Unicode path matching by searching both NFC and NFD-equivalent query forms.

## 0.1.22 — 2026-03-08
- Add `~/Library/CloudStorage` to default ignored paths to avoid initial indexing delays
- Make index building cancellable
- Fix unexpected indexing of files in ignored directories
- Fix stale selection affecting context menu targets
- Fix lost sort state and incorrect tooltip when search results are empty
- Speed up scanning/search by removing redundant index-map and ignore-path checks

## 0.1.21 — 2026-01-23
- Add shortcut `cmd+shift+c` for copy-paths
- Allow partial-quoted queries: `"Application Support"/**`
- Add drag-and-drop support for search input
- Faster wildcard search.
- Exclude more cloud storage paths from icon generation for better performance.
- Better experience for non-root monitor path
- Better experience for panel switching
- Fix duplicate results of globstar `**` search

## 0.1.20 — 2026-01-11
- Improve context menu for multiple selections.
- Add copy file(s) to clipboard support.
- Add shorthand aliases for `tag` and `infolder` filters.
- Hide the dock icon when the tray icon is enabled.

## 0.1.19 — 2026-01-10
- Add watch roots and ignore paths to settings.
- Avoid unnecessary rescan on FSEvent::Rescan.
- Handle Enter key in search input.

## 0.1.18 — 2025-12-15
- Add option for tray icon and defaults to disable
- Persistent cache when idle or enter background.
- Improve i18n locale detection and add zh-TW translation.
- Make tags filtering accepts multiple value.
- Supports `shift+arrow` to select multiple rows.
- Various UI/UX improvements and bug fixes.

## 0.1.17 — 2025-12-10
- Add `tag:` filter support so Finder tags can scope searches.
- Better i18n support.
- Improve the column resizing experience
- Improve the sorting experience

## 0.1.16 — 2025-12-08
- Improve sort order so directories are prioritized and folder size ranking stays stable.
- Refine selection handling for smoother keyboard and pointer interactions.
- Add an event column to the events panel for better debugging context.
- Fix the cursor state resetting incorrectly after Quick Look opens.

## 0.1.15 — 2025-12-06
- Implement double asterisk `**` in glob search.
- Implement history navigation with `ArrowUp`/`ArrowDown` for search bar.
- Add `Cmd+O` shortcut for file opening.
- Refined QuickLook animation positioning logic to better handle multiple monitor setups.
- Refined file row selection handling.

## 0.1.14 — 2025-12-03
- Make the results sortable(by name, path, size, create_time, modify_time)

## 0.1.13 — 2025-12-01
- 30% lower memory usage
- Quick Look is now fully native, with multi-file previews, smoother animations, and better alignment with macOS expectations. Thanks for [@Denis Stoyanov](https://github.com/xgrommx) for the help!
- Support `~` expansion in query path and filter
- Fix the database path so cache files land in the correct app config directory.

## 0.1.12 — 2025-11-27
- Allow double-clicking a result row to open the file immediately.
- Support wildcards in multi-path-segment queries for more flexible searches.

## 0.1.11 — 2025-11-25
- Implement `content:`, `nosubfolders:`filter
- Improve file selection and drag-drop support
- Cleaner app menu and context menu
- make ESC hide main window

## 0.1.10 — 2025-11-19
- Added new metadata filters (`dm:`, `dc:`, `type:`, `audio:`, `video:`, `doc:`, `exe:`, `size:`) for more precise searches.
- Reworked the parser/optimizer pipeline to flatten redundant AND/OR groups, collapse empty expressions, and reorder metadata filters for faster searching.
- Use the native context menu on right-click for a more consistent feel on macOS.

## 0.1.9 — 2025-11-17
- Speedup `parent:` and `infolder:` filters.

## 0.1.8 — 2025-11-16
- Cardinal now fully supports the "Everything syntax"(AND/OR/NOT, parentheses, quoted phrases, wildcards).
- Removed the legacy regex toggle and unified the search bar, hooks, and IPC payloads around the new parser pipeline.
- Highlight of search results was improved.
- Enhance show/hide shortcut.

## 0.1.7 — 2025-11-12
- Added a cancellable search pipeline for a more responsive search experience.
- Refined selected row styling with dedicated text color tokens for better contrast in both themes.

## 0.1.6 — 2025-11-11
- Further optimized search execution and reorganized the search cache for faster lookups.

## 0.1.5 — 2025-11-09
- search-cache: widen `NameAndParent` filename length tracking from `u8` to `u32` to handle very long paths without truncation or panic.

## 0.1.4 — 2025-11-09
- Fix i18n support for tray menu.
- Rescans now clickable while updating cache.

## 0.1.3 — 2025-11-08
- Added keyboard shortcuts for Quick Look (Space), Finder reveal (Cmd+R), copy path (Cmd+C), and refocusing search (Cmd+F).
- The search field auto-focuses after launch and whenever the quick-launch shortcut summons Cardinal.
- i18n: add Ukrainian language support and translations

## 0.1.2 — 2025-11-07
- feat(shortcut&tray): support global shortcut to toggle cardinal by [@Binlogo](https://github.com/Binlogo)
- feat(theme): implement theme switching functionality with user preferences
- feat(context-menu): add “copy filename” option and update translations
- feat(i18n): add Russian language support and translations

## 0.1.1 — 2025-11-07
- Fixes iCloud download triggered by thumbnail generation.

## 0.1.0 — 2025-11-07
