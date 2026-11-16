# Borrowing ledger — misterdownloadmanager (github.com/anefzaoui/misterdownloadmanager)

Read at v0.5.2 (~100k lines Rust, GPL-3.0-or-later). **License boundary: ideas and architecture only — zero code may come across.** JD2-vs-MDM positioning: MDM is an IDM-style manager (own engine, no host plugins, no linkgrabber plugin market; JD2 is a hoster-plugin framework). ZDownloader3 sits closer to JD2, but MDM solved several UX/security problems better than JD2 does.

Source layout consulted: `docs/ARCHITECTURE.md`, `docs/link-grabber.md`, `docs/queues-and-scheduler.md`, `docs/themes.md`, `crates/*` (engine, app, ipc, cnl, debrid, remote, mcp, extract, nmhost).

## High-value borrows (fits our design)

### 1. Click'n'Load done safely (its best idea)
JD2 exposes CNL2 on 127.0.0.1:9666 but blindly trusts posts and answers `jdcheck.js`. MDM keeps the port but:
- first post from a site → ask **Allow / Block / Not now**; answer is stored per *browser-named origin*, never the page's claim; unknown origin asked every time
- rate limit: ≤10 posts/min/site, ≤500 links, ≤256 KB/post, ≤20 pending posts
- links from pages are **never looked up or downloaded** until the user queues them ("Not looked up" state)
- SSRF blocklist: loopback, private/link-local ranges, `.local/.lan/.internal`, single-label names can never be queued
- doesn't answer `jdcheck.js` (sites may hide the button; honest trade-off)
- one program per port, and says so ("JDownloader is running…")
- toggle off = forget all site answers
→ **ZDownloader3 ships exactly this model for CNL2. It's the reference spec for our grabber daemon's security.**

### 2. Link look-ups with etiquette
On add, MDM HEAD-checks each link: shows *Online / Offline (404) / Needs login / Busy*, unticks offline, throttled per-site concurrency, and an option to disable look-ups entirely (some hosts count them as downloads). JD2's equivalent is the linkchecker, but MDM's "off by default? no — toggle + per-site caps + measured against user harm" framing is the right one.
→ queue these as opt-in toggles in Settings → Downloads.

### 3. Package semantics worth copying
- archive volumes (`movie.part1.rar`, `.7z.001`) auto-group into one package named after the archive
- "Already in the list" dedup section (unticked)
- package box edits name/save-to/password in-place; password kept **in memory only, never on disk**
- "Extract when done" checkbox when package holds an archive
→ folds into our `zd-core` model notes.

### 4. Explicit download-state etiquette
Failed items re-queue at the *end* with a 30s retry delay; queue stops when empty; sync queue for finished files; wake-on-schedule via RTC wakealarm for overnight queues.
→ our queue scheduler gets: retry-with-backoff + queue-drain semantics + (Linux) `/sys/class/rtc/rtc0/wakealarm` hook, no root needed beyond tmpfiles grant.

### 5. Secrets in the keychain
MDM stores debrid/proxy/site passwords as `keychain:<service>/<scope>/<key>` references in settings, written only after the keychain confirms; fallback plaintext used only when no keychain exists. JD2 stores some in plaintext JSON. We already planned keyring; adopt the *reference-in-settings* pattern verbatim (it sidesteps "which secrets go where" ambiguity).

### 6. Theme architecture: derived palettes + role keys
MDM themes are light/dark pairs declaring stable **role keys** (`button.hot`, `list.row`, …); a `derive.rs` builds all remaining roles from a few seeds in OKLab with WCAG contrast targets (4.5:1 text, 3:1 edges); style themes (Metro/Adwaita/Breeze/…) only pick control shapes; user theme files `inherits` a built-in and override only what they name.
→ **Direct upgrade for our 6 builtins**: keep libadwaita CSS as the render path, but generate Arc/Nord/Catppuccin palettes from seed triples with checked contrast, and let user themes be TOML files `inheriting` a builtin. GPL note: reimplement, do not port their palette tables — but the *role-key + derive + inherit* design is license-free.

### 7. Browser extension + native messaging host
Extension (MV3 Chromium/Firefox) → native messaging host → app; host accepts only MDM's own extension IDs; 1 MB message cap with paged transfers for large link lists (carries referrer/cookies per link, no isFirstPost spoofing possible); browser download canceled only after app accepts.
→ Phase-2 item for ZDownloader3 (post-MVP): `zd3-nmhost` + extension adds "Send to ZDownloader3" context item. Design constraint noted: referrer/cookies travel with the link — we must honor that for hoster plugins later.

### 8. Remote control as automation API, one surface
MDM exposes everything (list/add/start/stop/queues/settings) as one "automation API"; three frontends ride it: local IPC socket, LN-remote HTTPS server with pairing codes + device tokens (phone on LAN), and an MCP server for AI assistants (`rmcp`, stdio).
→ ZDownloader3's planned "remote control API" should be a single command bus + thin frontends (local socket, optional HTTPS+token, optional MCP), not three parallel APIs.

### 9. Headless mode as first-class
`--headless` runs the same `App` without egui, driven by the command channel. Makes server/Docker deployments trivial and forces the engine to never depend on the UI.
→ our `zd-core` stays UI-free by construction (already planned); add `--headless` to `zd3` CLI roadmap.

### 10. Testing & crash engineering
- `mdm-labserver`: local HTTP Range server with fault injection (slow/dropped/refused conns, no-range servers) + FTP/FTPS equivalents; engine end-to-end tests against it
- crash monitor out-of-process with minidumps + release symbols zip
- benchmark harness (`tools/bench`) producing the published speed claims
→ for our aria2 path: still worth a fault-injection test server around the RPC layer; if we ever self-build an engine, labserver is the model.

## Noted, not borrowing (scope/license/fit)

- **Own segmented HTTP/1.1 engine** (probe/planner/RAF-style workers/HostLimits): MDM's 5× IDM claim implies real wins vs single-connection, but aria2 already covers this class; building our engine is a research project, not a clone task. Decision point recorded below.
- **Torrents** (librqbit fork), **video pages** (yt-dlp), **galleries** (gallery-dl), **debrid** (real-debrid etc.): adjacent but separate products; debrid overlaps JD2 multihosters though — MDM's debrid REST client is a *better* starting point for our premium-host module than JD2's plugin approach for the top 5 services.
- **86 languages / i18n pipeline**, **website**, **crash minidumps**, **Windows-first polish** — process, not product; later or never.
- egui fork stack with per-platform rendering quirks — irrelevant; we chose GTK4/libadwaita.

## Decision points this raises

1. **aria2 vs own engine.** Borrow MDM's architecture only if we outgrow aria2's limits (per-host start intervals, mirror dedup across hosts, browser-like TLS fingerprinting for hostile sites). Not now.
2. **CNL2 spec above is the security baseline** — `zd-cnl` implements it before any GUI polish on the grabber.
3. **Command bus** becomes the integration seam for CLI, remote, MCP, and future extension host — design `zd-core::Command` from day one.

## Doc cross-links

- `docs/jd2-internals.md` — JD2 reference (engine, plugins, GUI, config)
- `docs/architecture.md` — ZDownloader3 module map + roadmap (needs the command-bus note added)
