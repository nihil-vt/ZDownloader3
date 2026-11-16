# JDownloader2 Internals — Decompiled Source Analysis

Analysis of shipped JD2 (Flatpak `org.jdownloader.JDownloader`, rev 50639, built 2026-04-22, Java 27). **Clean-room document: these are observations of design, not copied code.** ZDownloader3 is MIT/Apache-2.0 — no JD2 code may be reused verbatim.

Tooling: REA (archive inventory, hash verification, Evidence `ev_942c16a0…` / `ev_7c547c2d…`, bundle `/tmp/opencode/jd2-core-artifact.json`) + CFR 0.152 decompilation of `Core.jar`, `JDGUI.jar`, `JDHttp.jar`, `JDUtils.jar`, `cons.jar`, `Filters.jar`, `JAntiCaptcha.jar`, `Extraction.jar` → `/tmp/opencode/jd2src/` (2,833 files, 23 MB). REA is native/archive-oriented — Java bytecode is outside its analysis scope beyond inventory; decompile was done with CFR and is structurally complete but some methods were deliberately obfuscated (e.g. `handleCaptchaChallenge`) and `org.appwork.*` lives in separate jars not in the distro (framework not decompiled).

## 1. Component inventory (shipped)

| Jar | Classes | Role |
|---|---|---|
| Core.jar | 6,026 | Everything: plugins, controllers, GUI logic, APIs |
| JDGUI.jar | 1,114 (Swing layer) | `jd.gui.swing.*` — frame, tables, LAF, dialogs |
| JDHttp.jar | 87 | HTTP stack (`jd.http.Browser`, cookie jar, proxy selector, auth) |
| cons.jar | 44 | misc "cons" constants |
| Filters.jar | 257 | Filter engine (linkgrabber) |
| JAntiCaptcha.jar | 208 | JAC library (captcha solving) |
| Extraction.jar | 97+ | Optional extraction extension (7z/rar/zip, multipart) |
| JDUtils.jar | 57 | utils |
| libs/laf/* | — | FlatLaf (default LAF), Synthetica (licensed alternative) |
| themes/standard/ | 330 files | Single icon theme (291 PNG, 4 SVG), no theme engine |

## 2. Architecture overview

JD2 is layered as: **AppWork framework (external) → jd core (controllers/plugins) → Swing GUI (JDGUI) → optional extensions**. Communication is overwhelmingly **event-bus driven** (`*EventSender` per domain) plus **declared-config handlers** (`@DefaultIntValue` interfaces → `KeyHandler` statics) that the GUI binds to live.

### 2.1 Bootstrap sequence (`startup/Main` → `SecondLevelLaunch.mainStart`)

1. `Main`: logger redirection, `.jd_home` root, MyJD JSON mapper, jxbrowser load, `ParameterHandler`, LAF setup, `SecondLevelLaunch.mainStart`.
2. `SecondLevelLaunch.go()`: OS specials → JNA/tmpdir → uncaught-exception handler → TLS setup (BouncyCastle vs JSSE, `PREFER_BOUNCY_CASTLE_FOR_TLS`) → async browser globals thread → LAF on EDT → worker thread in `GUI_COMPLETE.executeWhenReached`: `HostPluginController.ensureLoaded()` → `PackagizerController.getInstance()` → `DownloadController.initDownloadLinks()` → `LinkCollector.initLinkCollector()` → `RemoteAPIController.getInstance()` → `MyJDownloaderController.getInstance()` → `ExtensionController.init()` → `AccountController` → `DownloadWatchDog.getInstance()` (starts download state machine) → clipboard monitor → `ChallengeResponseController.init()` → `JDGui.init()`.
3. Ordering matters: **UI comes up before plugins finish loading**; downloads auto-start only after `HOST_PLUGINS_COMPLETE` + list loaded (`DOWNLOADLIST_LOADED`).
4. State gating via `SingleReachableState` (appwork) instead of callbacks: `GUI_COMPLETE`, `INIT_COMPLETE`, `HOST_PLUGINS_COMPLETE`, `ACCOUNTLIST_LOADED`, `EXTENSIONS_LOADED`.

### 2.2 Configuration system

Declared-interface pattern throughout:

```java
public interface GraphicalUserInterfaceSettings extends ConfigInterface { … }
// settings/staticreferences/CFG_GUI.java
public static final KeyHandler GUI_SIDEBAR_VISIBLE = StorageHandler.instance().getKeyHandler("LinkgrabberSidebarVisible", …);
```

- Interfaces carry `@DefaultIntValue`/`@DefaultEnumValue`/`@DescriptionForConfigEntry`/`@RequiresRestart` annotations; `JsonConfig.create(iface)` generates a proxy; `CFG_*` static holders expose named `KeyHandler`s.
- Persisted as `cfg/<fqcn>.json` (e.g. `cfg/org.jdownloader.settings.GraphicalUserInterfaceSettings.json` — the live dump we copied has ~200 GUI toggles).
- Legacy plugin config: `SubConfiguration` (AES-obfuscated JSON `.ejs`, hardcoded key — not real encryption), being replaced by plain-JSON `cfg/plugins/<id>` via `PluginJsonConfig`.
- GUI listens via `GenericConfigEventListener.onConfigValueModified` → relayout. Example: toggling `linkgrabbersidebarvisible` re-flows the panel immediately.
- **Lesson for ZDownloader3:** adopt the same *declared settings* pattern (e.g. a Rust `Settings` trait + typed keys + change events), plain JSON or SQLite, no hidden encryption.

### 2.3 Event system

Every subsystem publishes via `*EventSender`: `DownloadWatchdogEventSender`, `DownloadControllerEventSender`, `LinkCollectorEventSender`, `LinkCrawlerEventSender`, `ReconnectEventSender`, `ChallengeResponseEventSender`, `FileCreationEventSender`, `PackagizerControllerEventSender`, `IPControllEventSender`, `AccountCheckerEventSender`, … GUI subscribes directly; RemoteAPI wraps the same events as `EventPublisher`s streamed over websocket/long-poll to MyJDownloader clients. **ZDownloader3: tokio `broadcast` per domain.**

## 3. Download engine

### 3.1 Models

- **`DownloadLink`** (`jd/plugins/DownloadLink.java`, 2,329 lines): URL variants (`getDownloadURL`/`getContentUrl`/`getCustomUrl`/`getContainerUrl`), filename, size (may be unknown −1), chunk count, priority, properties map, `LinkStatus`/`SkipReason`/`ConditionalSkipReason`.
- **`FilePackage`** vs **`CrawledPackage`**: same shape, different phase; views toggle between them. `FilePackage` persisted via `DownloadControllerStorable` (snapshot JSON + rezip).
- **`CrawledLink`** wraps `DownloadLink` pre-queue and its `FilePackage` parent.

### 3.2 Pipeline

```
add text/URLs
  → LinkCollectingJob (origin/source tracking, LinkOrigin)
  → LinkCollector.TIMINGQUEUE          [linkcollector/LinkCollector.java, 3,315 lines]
  → LinkCrawler (thread pool)          [linkcrawler/LinkCrawler.java, 31 classes]
       PluginForDecrypt plugins walk the tree (deep crawl, regex rules, container extraction)
       → CrawledLink / CryptedLink
  → LinkCollector (grabber list, filters via LinkGrabberFilterRule + PackagizerController rules)
  → user confirms / auto-add
  → DownloadController (FilePackages/DownloadLinks, persisted)
  → DownloadWatchDog (scheduler, SingleReachableState RUNNING/PAUSED/STOPPED)
       DownloadLinkCandidateSelector picks candidates (mirror detection via hash/size,
       per-plugin max-connections, disk-space reservation via DiskSpaceManager)
  → SingleDownloadController → fresh PluginForHost instance + AccountCache entry
  → plugin.handle(link, account) → jd.http.Browser → DownloadInterface (RAFDownload w/ chunks)
  → FinalLinkState → FileCreationEventSender.NEW_FILES → Extraction extension (optional)
```

Key behaviors:
- Per-host concurrent limit: `getMaxSimultanDownloadNum()` (default 1 free) returned by the plugin; per-account when premium. `getTimegapBetweenConnections`, start-interval queue with progress (not sleep).
- Skip reasons are typed, not strings: `WaitingSkipReason`, `WaitForAccountTrafficSkipReason`, `multihost offline` mapping to `PluginException(2048)` — UI shows localized reasons.
- Downloads are **resumable** via RAF split-range `Chunks`; mirror candidates dedup by size/hash.
- `DownloadLink.PROPERTY_ARCHIVE_ID` links chunks of multipart archives to the extractor.

### 3.3 Packagizer / Filter engine

Two-stage rule chain applied inside `LinkCollector` after crawl:

1. **`org.jdownloader.controlling.filter`** (`LinkGrabberFilterRule`, `LinkFilterController`) — accept/except by URL regex, filetype, size, online status, plugin status, origin; drops garbage before it shows up.
2. **`org.jdownloader.controlling.packagizer`** (`PackagizerController`, `PackagizerRule`, JSON-config ordered, `stopAfterThisRule`) — rename (`<jd:packagename>`, `<jd:simpledate:>`, env vars), move to package, set destination, chunks/priority/autostart/autoextract, subfolder rules.

**ZDownloader3: port the *rule model* (condition + action, ordered list, stop flag), not the code. It's the highest-value part of the linkgrabber besides plugins.**

### 3.4 Accounts

`AccountController` (singleton) refreshes `AccountInfo` (traffic left, valid-until, multihost list) on a schedule; `Account` carries cookie store + traffic factor per `MultiHostHost`. `AccountCache` pre-assigns accounts per candidate link (original-host preferred, else multihoster, `ACCOUNTTYPE.NONE/ORIGINAL/MULTI`); plugins call `update(link, account, bytes)` to deduct traffic, `enoughTrafficFor` throws typed skip reasons. Premium-login captchas go through the same challenge pipeline.

## 4. Plugin system (the product's moat)

Annotations define everything (`@HostPlugin`/`@DecrypterPlugin` with `names()`, `urls()` regex, `interfaceVersion()`, `revision()`):

| Class | Role |
|---|---|
| `jd/plugins/Plugin.java` (1,304) | Base: `PluginBrowser`, captcha challenge creation, config access, file helpers |
| `PluginForHost.java` (3,815) | Hoster downloads: `handleFree`/`handlePremium`/`handleMultiHost` dispatch via `handle(link, account)`; waits, retries, captcha on premium login, account traffic |
| `PluginForDecrypt.java` (806) | Crawlers: `decryptIt(CryptedLink)`; distributes results via `LinkCrawlerDistributer` (with rate-limit delayer) |
| `LazyPlugin`/`PluginWrapper` | Metadata shells; real classes loaded lazily by child-per-plugin `PluginClassLoader`s |
| `PluginController` + `PluginScannerFiles` | Hash-scan plugin dirs, cache, verify `revision()` + `PluginDependencies` freshness |
| `host/PluginFinder` | host→plugin map with broken-plugin blacklist |
| `ContainerPluginController` | DLC/CCF/RSDF container import |

Plugins are **shipped out-of-band via the update service**, not in Jars — the decompiled tree contains the *framework*, not the ~thousand host plugins (those live on JDownloader's CDN, versioned by `revision()`). They execute in-process with full JVM rights (no sandbox).

Captcha chain (`org/jdownloader/captcha/v2/`): `Challenge<T>` (basic/click/multiclick/recaptcha/turnstile) → `ChallengeResponseController.handle()` → blacklist check → `createList()` of applicable `ChallengeSolver`s → per-solver executors → manual dialog solvers, JAC/JAntiCaptcha, 9kw, DBCH, EndCaptcha, ImageTyPerz, TwoCaptcha — with quality-assurance feedback loop (`CaptchaQualityEnsuranceRule`) and per-solver threading. Plugin calls `getCaptchaCode()`/`handleCaptchaChallenge()`.

## 5. GUI layer

### 5.1 Composition (`JDGui` singleton)

`MigLayout("ins 0, wrap 1, [grow,fill]", "[grow,fill]0[shrink]")`: `MainTabbedPane` center + `StatusBarImpl` dock SOUTH; `MainToolBar` dock NORTH; custom closable tab headers (`TabHeader`, `DonateTabHeader`, `PromotionTabHeader`); macOS full-window-content titlebar tweaks under FlatLaf. Title pattern config (`#TITLE #SPEED #RUNNING_DOWNLOADS #UPDATENOTIFY`) re-evaluated every 1s.

Tabs: **Downloads**, **Linkgrabber** (always), optional closable **Settings** and **MyJDownloader**; plus donation/promo tabs (we will not clone).

### 5.2 Shared table stack (`gui/views/components/packagetable/`)

One generic `PackageControllerTable<P,C>` + `PackageControllerTableModel` used by both main views: hierarchical package rows + child link rows, `ExtColumn` subclasses (Size, Speed, ETA, Progress, Hoster, Availability, Task, date columns…), filter chain with `DelayedRunnable` coalescing, selection guessing across model recreations via `UniqueAlltimeID`, move up/down/top/bottom, ALT-arrow navigation, drag-drop merge. Selection context passed to action classes as `SelectionInfo<Package,Link>`; all menu/toolbar/bottombar items are `ContextMenuManager` models rendered by `MenuBuilder`/`CustomizeableActionBar` — **configurable toolbars as data**.

### 5.3 Linkgrabber panel

`MigLayout`: table + optional right `LinkGrabberSidebar` (quick-filter tables for hoster/type/online-status with checkbox toggles) below/beside `LinkGrabberWidgetContainer`: `LinkgrabberOverview` (aggregate stats) + `LinkgrabberProperties` (per-link/package edit: filename, URL, size, checksum, passwords, comment, save-to). Bottom bar: clear-offline/disabled, search, confirm button dropdown (all/selected × autostart/force), QuickSettings submenu, auto-confirm stop.

### 5.4 Downloads panel

Same table + overview header (`DownloadOverview`: packages/links/enabled/disabled counts, bytes totals/remaining, speed, ETA, finished/failed — every entry individually toggleable via config keys `overviewpanel*`) + properties panel (filename/save-to/checksum/download password/archive password/comment rows, toggleable). Bottom bar: delete split-button, search, quick filters, chunks/parallel downloads/max-parallel-per-host editors, speed limit, resume/reset, add-links.

### 5.5 Status bar (`StatusBarImpl`)

Left: scrollable premium account chips (`premiumbar.ServicePanel`, traffic + state per host) → disabled-status label → `ReconnectProgress` (while reconnecting) → dynamic `IconedProcessIndicator`s: `CaptchaIndicator` (one per pending solver job), `JobLinkCrawlerIndicator`, `LinkCheckerIndicator`, `AutoConfirmProcessIndicator`. Updates live on `AccountControllerEvent`/`ChallengeResponseEvent`.

### 5.6 Settings UI

`ConfigurationView` tab → `ConfigSidebar` (JList of panels) + panel container; panels are `ConfigPanel`s that reflect `SubConfiguration`/`ConfigContainer` descriptors into widgets; plus **declared** `ConfigInterface`s rendered via `Factory`. Entire settings UI is data-driven from the config interfaces — adding a setting = adding an interface method.

### 5.7 Theming

FlatLaf (default) or Synthetica (licensed), installed by `LookAndFeelController`; `LAFOptions` is the central color/gap facade (package-row highlight, table selection, panel background, insets). Icons: `NewTheme.I().getIcon()` from `themes/standard/` (291 PNGs + few SVGs). **No CSS/stylus infrastructure — it's all Java code.** ZDownloader3 replaces this with libadwaita CSS (already scaffolded with 6 builtins).

### 5.8 Notifications & tray

`BubbleNotify` → `Balloner` (per-screen anchored frames, slide/fader animation, silent-mode aware); bubble types: crawler-finished, update, reconnect, captcha, download start/stop/pause. `TrayExtension` (`java.awt.SystemTray`): flash taskbar, minimize/close-to-tray, per-bubble enable config.

## 6. Remote APIs & update/distribution (reference only)

- `RemoteAPIController`: local JSON/websocket API (HTTP on 3128 default-off, localhost-only) + CNL2 extern interface (9666) + **MyJDownloader cloud** (`api.jdownloader.org`, custom `gazeisp` AES+gzip over HTTP, appKey-authenticated, device-bind flow). MyJD pushes captcha challenges/answers and mirrors the whole RemoteAPI surface (`bindings/interfaces/*` mirror `@ApiNamespace`).
- `UpdateController` loads in the updater module (not shipped in these jars); extensions (Extraction, EventScripter, FolderWatch, Scheduler…) install on demand via it. No signature verification exists for plugins/extensions in the shipped code — trust = the update endpoint.
- Plugin trust model: unsigned plugins, full JVM access. **Not acceptable for us**: ZDownloader3 plugins should be sandboxed (separate process or at minimum capability-declared) or at least signed.

## 7. Prioritized takeaways for ZDownloader3

1. **Adopt the declared-settings pattern** (typed keys + change events). It is the single best structural idea in JD2.
2. **One package-table component, two tab bindings** (linkgrabber/downloads) — implement once as a `TreeListModel`+`ColumnView`.
3. **Event bus per domain** with GUI subscribing directly; remote API is just another subscriber.
4. **LinkGrabber pipeline = crawl → filter → packagizer → staged store → queue**; filters and packagizer rules are the same condition/action engine — build one rule engine used by both.
5. **Plugin API = annotated registry + metadata shells + lazy classloaders**; in Rust: registry of trait objects + manifest; subprocess isolation if feasible later.
6. **Status bar = premium chips + per-process indicators**; recreate with symbolic icons.
7. **Skip the Swing/LAF layer entirely** — libadwaita covers it.
8. **Do not copy**: donation/promo tabs, clipboard-on-by-default, unsigned in-process plugins, `SubConfiguration` obfuscation, MyJDownloader "gazeisp" custom crypto, title-bar spam.
9. **Copy behaviorally**: overview panel toggleability, confirmation UX (auto-confirm with countdown), mirror dedup, traffic-aware waits, typed skip reasons with localized messages, event-driven everything.

## 8. What was NOT decompiled (bounds of this analysis)

`org.appwork.*` (config/storage/logging/event base/ShutdownController), updater module (`updatev2.UpdateController`), plugin repository (plugins ship over the air), MyJDownloader server side, Synthetica internals, 3rd-party libs (BouncyCastle, Rhino, 7z binding, jna). Conclusions about those are inferred from call sites only.

*Evidence ledger: REA inventory `ev_7c547c2d…`, artifact `ev_942c16a0…`; CFR outputs `/tmp/opencode/jd2src/`; live config capture `~/.var/app/org.jdownloader.JDownloader/data/jdownloader/` (user-installed flatpak, untouched).*
