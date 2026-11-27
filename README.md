> **Community fork:** this repository is based on [cardisoft/cardinal](https://github.com/cardisoft/cardinal). The default branch **`public/cardinal-enhancements`** contains this fork's complete feature set, including contributions from [EminSet/cardinal](https://github.com/EminSet/cardinal). [Türkçe: fork differences and integration guide](docs/FORK-INTEGRATION.tr-TR.md).

## Working with this fork

```bash
git clone --branch public/cardinal-enhancements https://github.com/MehmetEminBozdag/cardinal.git
cd cardinal
```

This branch adds visual type/date/size filters, newest-first sorting, saved searches, favorites, search-coverage controls, expanded native menus, in-app Move to Trash, APFS-aware duplicate analysis, and bounded recovery after a WebKit renderer failure. Added UI strings and the 0.1.24 release notes cover all 15 supported locales. Core query-matching speed is not changed; performance work concerns large-result metadata sorting, cancellation, and request backpressure.

The latest package is [v0.1.24-fork.2](https://github.com/MehmetEminBozdag/cardinal/releases/tag/v0.1.24-fork.2) (2026-11-27), for Apple Silicon. In addition to the features above, it includes:

- **Sizes:** folder totals, a treemap, free disk space, Trash usage, and cleanup suggestions. Allocated and logical sizes can differ; a folder total is not a guarantee of reclaimable APFS space.
- **Applications:** uninstall workflows and suggestions for possible leftovers from apps no longer installed. Review suggested paths before approving a move to Trash.
- **Operations:** a persistent, rotated history of Move to Trash operations; this is a log, not an undo system.
- **Background CPU control:** size scanning runs while the Sizes tab is selected and Cardinal is in the foreground. Moving to another tab suspends it immediately. When Cardinal goes into the background with Sizes selected, scanning may continue for **15 seconds**, then full and incremental scans are cancelled. Pending changes are retained for resuming; ordinary filename indexing remains active independently.

These additions incorporate EminSet's work through [`a4658a1`](https://github.com/EminSet/cardinal/commit/a4658a17ffabd708dda08508aee93789f3aeaa30), followed by this fork's size-scan activity controls and regression tests. See the [changelog](CHANGELOG.md) for the release scope and verification limits.

For integration, start with the complete branch rather than combining the three `contrib/*` review branches. Those branches remain for existing PR reviews. The fork still uses the upstream bundle identifier `com.cardinal.one`; give a separate product its own identifier, storage paths, shortcuts, and signing configuration before distributing it alongside Cardinal. See the [integration guide](docs/FORK-INTEGRATION.tr-TR.md) for source mappings, IPC boundaries, limitations, branch history, validation, and the upstream status checked on **2026-11-27**.

Upstream `master` is still at `4c50734f9a09d88110f96652b43634b412f79449`; [our PR #228](https://github.com/cardisoft/cardinal/pull/228) is open and unmerged. Open third-party PRs are proposals, not features shipped by this fork. The app's native version is `0.1.24`; that number alone does not identify this branch or prove an official upstream release.

---

<div align="center">
  <img src="cardinal/mac-icon_1024x1024.png" alt="Cardinal icon" width="120" height="120">
  <h1>Cardinal</h1>
  <p>macOS file search, based on Cardinal with community workflow and reliability enhancements.</p>
  <p>
    <a href="#using-cardinal">Using Cardinal</a> ·
    <a href="#building-cardinal">Building Cardinal</a>
  </p>
  <img src="doc/pub/UI.gif" alt="Cardinal UI preview" width="720">
</div>

---

[English](README.md) · [Español](doc/pub/README.es-ES.md) · [한국어](doc/pub/README.ko-KR.md) · [Русский](doc/pub/README.ru-RU.md) · [简体中文](doc/pub/README.zh-CN.md) · [繁體中文](doc/pub/README.zh-TW.md) · [Português](doc/pub/README.pt-BR.md) · [Italiano](doc/pub/README.it-IT.md) · [日本語](doc/pub/README.ja-JP.md) · [Français](doc/pub/README.fr-FR.md) · [Deutsch](doc/pub/README.de-DE.md) · [Українська](doc/pub/README.uk-UA.md) · [العربية](doc/pub/README.ar-SA.md) · [हिन्दी](doc/pub/README.hi-IN.md) · [Türkçe](doc/pub/README.tr-TR.md)

## Using Cardinal

### Download

**The commands and upstream releases below install original Cardinal, not this fork.** Download this fork's Apple Silicon DMG from [the latest fork release](https://github.com/MehmetEminBozdag/cardinal/releases/latest), or build `public/cardinal-enhancements`. This is the published community fork build with an ad-hoc signature, not an Apple-notarized upstream build. The release tag identifies the fork snapshot; the bundled app still reports native version `0.1.24`.

Install original Cardinal with Homebrew:

```bash
brew install --cask cardinal-search
```

You can also grab the latest packaged builds from [GitHub Releases](https://github.com/cardisoft/cardinal/releases/).

### i18n support

Need a different language? Click the ⚙️ button in the status bar to switch instantly.

### Search basics

Cardinal now speaks an Everything-compatible syntax layer on top of the classic substring/prefix tricks:

- `report draft` – space acts as `AND`, so you only see files whose names contain both tokens.
- `*.pdf briefing` – filter to PDF results whose names include “briefing”.
- `*.zip size:>100MB` – search for ZIP files larger than 100MB.
- `in:/Users demo !.psd` – restrict the search root to `/Users`, then search for files whose names contain `demo` but exclude `.psd`.
- `tag:ProjectA;ProjectB` – match Finder tags (macOS); `;` acts as `OR`.
- `*.md content:"Bearer "` – filter to Markdown files containing the string `Bearer `.
- `"Application Support"` – quote exact phrases.
- `brary/Applicat` – use `/` as a path separator for sub-path searching, matching directories like `Library/Application Support`.
- `/report` · `draft/` · `/report/` – wrap tokens with leading and/or trailing slashes to force **prefix**, **suffix**, or **exact** name matches when you need whole-word control beyond Everything syntax.
- `~/**/.DS_Store` – globstar (`**`) dives through every subfolder under your home directory to find stray `.DS_Store` files anywhere in the tree.

For the supported operator catalog—including boolean grouping, folder scoping, extension filters, regex usage, and more examples—see [`doc/pub/search-syntax.md`](doc/pub/search-syntax.md).

### Keyboard shortcuts & previews

- `Cmd+Shift+Space` – toggle the Cardinal window globally via the quick-launch hotkey.
- `Cmd+,` – open Preferences.
- `Esc` – hide the Cardinal window.
- `ArrowUp`/`ArrowDown` – move the selection.
- `Shift+ArrowUp`/`Shift+ArrowDown` – extend the selection.
- `Space` – Quick Look the currently selected row without leaving Cardinal.
- `Cmd+O` – open the highlighted result.
- `Cmd+R` – reveal the highlighted result in Finder.
- `Cmd+C` – copy the selected files to the clipboard.
- `Cmd+Shift+C` – copy the selected paths to the clipboard.
- `Cmd+F` – jump focus back to the search bar.
- `ArrowUp`/`ArrowDown` (in search bar) – cycle search history.

Happy searching!

---

## Building Cardinal

### Requirements

- macOS 12+
- Rust toolchain
- Node.js 18+ with npm
- Xcode command-line tools & Tauri prerequisites (<https://tauri.app/start/prerequisites/>)

### Development mode

```bash
cd cardinal
npm ci
npm run tauri dev -- --release --features dev
```

### Production build

```bash
cd cardinal
npm ci
npm run tauri build
```

Use the toolchain pinned in `rust-toolchain.toml` (`nightly-2025-12-11` at this snapshot). Build outputs are under `cardinal/src-tauri/target/release/bundle/` relative to the repository root. They are ignored by Git. See the [guide's validation commands](docs/FORK-INTEGRATION.tr-TR.md#dogrulama) before integrating or distributing a build.
