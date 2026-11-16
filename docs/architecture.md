> **Project direction changed 2026-11-16:** ZDownloader3 is now a **GPL-3.0 fork of Mister Download Manager** (see `NOTICE.md`), not a from-scratch MIT/Apache reimplementation. The module map below describes the abandoned clean-room plan and is kept for reference; the live plan is in `attic/` (history) and `docs/mdm-borrow.md`.

# ZDownloader3 — Architecture & JD2 Analysis

Status: design phase. Analysis target: **shipped JDownloader2** (Flatpak `org.jdownloader.JDownloader`, build rev `JDownloaderRevision: 50639`, buildDate 2026-04-22, Java 27 runtime), obtained by running the official Flatpak once to self-update. Observed, not executed beyond its normal startup path.

Licensing: ZDownloader3 is **MIT OR Apache-2.0, clean-room**. JD2 is closed-source freeware (`LicenseRef-proprietary`); its core is partially viewable as compiled classes but no JD2 (or GPL/AGPL, e.g. pyload) code is copied. This document records *observed behavior and structure* for reimplementation.

## 1. Evidence base

| Source | Notes |
|---|---|
| `Core.jar` (9.1 MB, sha256 e581f214…) | 6026 classes; REA `inspect_artifact` Evidence `ev_942c16a0…`, inventory Evidence `ev_7c547c2d…` (6371 entries verified); exported bundle `/tmp/opencode/jd2-core-artifact.json` |
| `JDGUI.jar` (2.8 MB, 1160 files) | Swing widget layer (`jd.gui.swing.*`) |
| `libs/laf/` | `flatlaf.jar`, `synthetica*.jar` — JD2's look-and-feels are FlatLaf and Synthetica |
| `themes/standard/` | 330 files (291 PNG icons, 4 SVG, 1 WAV) — single "standard" theme, no theme engine in the OSS sense |
| `cfg/*.json` | Live config dump: every GUI toggle, overview-panel visibility flags, hotkeys, filters |
| `build.json` | Component revisions (AppWork 4055, update client 1061, …) |
| Class/bytecode inspection | `javap` signatures + constant pools of `JDGui`, `JDownloaderMainFrame`, `LinkGrabberPanel`, `DownloadsPanel` |

Confidence notes: class counts, package layout, config keys and settings surface are **observed**. Layout mechanics (FormLayout/MigLayout, docking) are **observed at bytecode constant-pool level**. Pixel-level UX is **inferred** from component names and config flags; not all of it was seen running. Limitations: Java archives were inventoried as ZIP containers by REA; no Java decompiler was run on method bodies; behavioral claims about hotkeys/dialogs need runtime confirmation later.

## 2. What JD2 is (as designed)

JD2 = **Swing desktop app** (not SWT/JavaFX), built on:

- **AppWork framework** (`org.appwork.*`, closed): logging, config (`*.json` + `KeyHandler`s), dialogs, events (`EventSender`), storage, OS utils, reconnect, unpack.
- **FlatLaf** as default modern LAF (`lookandfeeltheme: "DEFAULT"`, macOS transparent titlebar tweaks via FlatLaf client properties), **Synthetica** as alternative.
- **Package model** everywhere: `DownloadLink` ⊂ `FilePackage`; LinkGrabber side mirrors it (`CrawledLink` ⊂ `FilePackage`).
- **Plugin engine**: hosts (`*.hoster.*`) + crawler plugins under `org/jdownloader/plugins/` (1773 classes), updated over the air with the core.
- **MyJDownloader** cloud remote-control API (`org/jdownloader/api/`), plus a local **JDRemoteAPI** (JSON).
- Update system with `update/versioninfo`, `libs/laf` pluggable, extensions (e.g. Extraction).

## 3. GUI/UX structure (the part we care about first)

### 3.1 Window anatomy

`JDownloaderMainFrame` (Swing `JFrame`)
├─ `JDMenuBar` (menu: File/Downloads/Help…, `ChunksEditor`, `SpeedlimitEditor`, `ParalellDownloadsEditor`)
├─ content pane (FormLayout)
│  ├─ center: `MainTabbedPane` — closable custom tab headers (`TabHeader`, `ClosableTabHeader`)
│  │   ├─ **Linkgrabber tab** (`LinkGrabberView`) — "before queue" staging area
│  │   └─ **Downloads tab** (`DownloadsView`) — active/history queue
│  │   (+ optional MyJDownloader / Donate tabs)
│  └─ SOUTH dock: `StatusBarImpl` (premium accounts summary, speed meter, reconnect, captcha indicator, donation button)
├─ `MainToolBar` (configurable action bar: Start/Stop/Pause downloads, reconnect, speed limiter, clipboard toggle, silent mode, settings, update)
├─ `TrayExtension` (tray icon, silent mode, flash-on-event)
└─ left/right `CustomizeableActionBar`s per panel + `LinkgrabberWidgetContainer` / `WidgetContainer`

Title bar doubles as a live indicator: `updateTitle` weaves `#SPEED`, running-download count, and update-notify into `#TITLE` (`speedinwindowtitle: "WHEN_WINDOW_IS_MINIMIZED"`, pattern `|#TITLE|| - #SPEED/s|| - #UPDATENOTIFY|`).

### 3.2 The two core views (the actual product)

Both are **package-table views** sharing `PackageControllerTable` (32 classes: sortable hierarchical table, package rows + child link rows, filters, rename-in-place, clipboard, drag-drop, context menus).

**LinkGrabberPanel**
- `LinkGrabberTable` — the crawled-but-not-yet-queued links, grouped by package
- `LinkGrabberSidebar` (+`LinkgrabberWidgetContainer`) — right sidebar: properties of selected link/package, quick-filter **widgets** (hoster filter, online-status filter, file type), overview panel
- bottom bar (`bottombar/` package, ~`CustomizeableActionBar`): confirm/accept links, auto-confirm actions, search field
- 941 classes under `gui/views/linkgrabber` overall incl. context menus, add-links dialog (35 classes!), overview widgets

**DownloadsPanel**
- `DownloadsTable` — same package-table pattern, richer columns (Task, Connection, Speed, ETA, Hoster, Added…)
- Overview panel (`overviewpanel/`): live stats header — running/finished/failed counts, total bytes, ETA, speed, package/link counts — each visibility-toggleable (see config keys `overviewpanel*`)
- `bottombar/`: search field, quick filter menu items
- properties panel (`properties/AbstractNodePropertiesPanel`, 27 classes): filename, save-to/download-from, checksum, passwords, comment

### 3.3 Window-level behaviors worth copying or deliberately dropping

- **Two tabs, one table component** — linkgrabber and downloads are the same UI twice with different data sources and actions. → ZDownloader3: one `PackageTable` widget, two model bindings.
- **Everything is configurable** — toolbars, bottom bars, overview visibility, title pattern, hotkeys. → good defaults first; configurability second.
- **Title-bar speed readout**, **tray + silent mode**, **taskbar flash on captcha/finish**.
- **Clipboard monitoring** (auto-grab links on copy) — off by default for us? JD2 has `clipboardmonitored:true`, `clipboardskipmode:"ON_STARTUP"`. → keep opt-in.
- **Synthetica/FlatLaf + 330-icon theme** — we replace with libadwaita + 6 builtin CSS themes (already scaffolded).

## 4. aria2 mapping (what we get free vs. what we build)

| JD2 concept | aria2 (1.37) native | ZDownloader3 action |
|---|---|---|
| File download, multi-connection split | `split`, `max-connection-per-server`, HTTP range | reuse; set fast defaults (16 splits, 16 conns) — "optimized settings built in" |
| Queue, per-host limits | `max-concurrent-downloads`, `max-connection-per-server` | per-host rule table *above* aria2; aria2 has no per-host delay |
| Download list / packages | none | **our model layer**: `Package{id,name,save_to,links[]}` persisted in SQLite; aria2 GID ↔ link id |
| Linkgrabber (crawl, filter, stage) | none (direct URLs only) | **our crawler pipeline**: input → host plugin `crawl()` → filter chain → staging store |
| Reconnect (router IP change) | none | plugin-like reconnect script runner (FRITZ/moden config via shell), optional |
| Captcha | none | hook point; manual captcha dialog + optional external solvers (user's own keys) |
| Premium accounts | none | account store (host → credentials/traffic), required headers/cookies injection |
| Speed limiter, pause/stop | `max-overall-download-limit`, `pause` | direct RPC |
| Extraction (7z/zip/multi) | none (needs sevenzip) | later: `p7zip` crate bindings; stub |
| Auto-resume on failure, mirrors | `continue`, `uri-selector` | reuse |
| History of finished/failed | aria2 has none persistent across restarts | SQLite history in our layer |

**Gaps we must build:** linkgrabber pipeline, host/crawler plugin system, package abstraction, account manager, UI. **We do not rebuild:** HTTP engine, chunking, checksums, resume.

## 5. Proposed module map

```
zdownloader3/                 workspace
├─ crates/zdownloader3        GTK4/libadwaita app shell (done: window, tabs, theme picker)
│                             adds: PackageTable widget, sidebar, status bar, prefs dialog
├─ crates/zd-core             model: Package, Link, DownloadState, events (tokio broadcast)
│                             queue policy, filter chain, SQLite persistence (sqlx)
│                             command bus (`zd_core::Command`) — seam for CLI/remote/MCP/extension
├─ crates/zd-aria2            JSON-RPC client (done: getVersion) → +addUri, tellActive,
│                             tellStatus, pause/unpause, remove, getGlobalStat, tellWaiting
├─ crates/zd-plugin-api       trait HostPlugin/CrawlerPlugin + manifest (name, match hosts),
│                             lifecycle hooks (can_handle, crawl, handle_captcha?)
├─ crates/zd-plugin-hosts     builtin plugins (none at first; API + tests)
├─ crates/zd-accounts         my/premium account store + per-host rules
└─ crates/zd-gui-table?       (only if table outgrows main crate)
```

UI language of the clone: header bar with tabs (`AdwTabView` — matches scaffold), package table = `ColumnView`/`ListView` with expandable package rows (GTK4 has tree-expandable `ListStore` models, or two-level `TreeListModel`). Status bar at bottom (`AdwBanner`-like custom bar), tray via `StatusNotifierItem` (ksni).

## 6. Theme system (scaffolded, this lands)

Builtin: Adwaita Light/Dark, Arc, Nord, Catppuccin Mocha/Latte. CSS `@define-color` overrides on libadwaita named colors + one provider slot with remove-on-switch (already implemented). Later: per-app accent, high-contrast option, icon set = symbolic icons only (no JD2-style 291-PNG theme; libadwaita symbolic icons + a small custom set).

## 7. Roadmap

1. **Scaffold** ✅ (window, themes, aria2 stub, tests)
2. **zd-core model + SQLite** — packages/links/state, event bus, unit tests
3. **zd-aria2 → addUri/tellStatus/global stat**, poll loop in glib future; replace status-bar stub with live speed
4. **Package table UI** — one widget, two tabs; row status icons, selection, context menus
5. **Linkgrabber pipeline** — paste/clipboard add, dedup, SQLite staging, "start downloads" moves to queue
6. **Plugin API + 1 reference plugin** (e.g. generic HTTP directory listing crawler — proves the API without duplicating JD2 hosts)
7. **Packaging** — `.desktop` ✅, metainfo, flatpak manifest (we already know the flatpak layout works), install docs
8. Later: accounts, reconnect, extraction, remote-control API

## 8. Notable JD2 behaviors to NOT copy blindly

- Donation/promo tabs & banner ads (`bannerenabled`, sponsor/promo views, affiliate host PriorityTab?)
- Global clipboard monitor defaulting on; password/email stored in plaintext cfg (JD2 stores some creds in json — we use keyring)
- Java/Swing custom painting everywhere → ours: native libadwaita widgets
- Over-aggressive update nag

## 9. Immediate next actions

1. Implement `zd-core` model + `zd-aria2` full RPC (steps 2–3) — needs decision: **SQLite (sqlx) vs. plain JSON files?** JD2 uses json+zip snapshots; we should use SQLite.
2. Design the `ColumnView`-based package table or prototype with AdwExpanderRow? (GTK4 `ColumnView` lacks tree mode → use `TreeListModel` around flat `Link`/`Package` nodes.)
3. Runtime pass: run JD2 (flatpak) briefly with a test download to capture tab layout screenshots for `docs/` (needs user consent for running with network).
