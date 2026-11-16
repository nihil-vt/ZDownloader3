# zDownloader³ — fork roadmap

Overrides the upstream `ROADMAP.md` (kept as reference). Guiding principle: **JDownloader2 functionality and look, Linux-native, GPL-3.0 fork hygiene** — upstream-compatible changes stay mergeable.

Legend: ✅ done in fork · 🔄 in progress · 📋 planned · ⚠️ blocked / needs a decision

## UI/UX parity with JD2

- ✅ Two tabs: `Downloads` / `Linkgrabber`, flat tabs under the toolbar
- ✅ JD2-style theme (light/dark) — default on Linux; toolbar icons JD2-tinted (green/red)
- ✅ Package-grouped list with tickboxes, look-up status, `Check/Uncheck all`
- ✅ `Name | Hoster | Size | Status` columns; package-row tint like JD2's "packages background highlight"
- ✅ Linkgrabber packages auto-grouped by folder across hosts, and by archive volume names (JD2 packs by folder; JD2's archive rule likewise ignores host)
- ✅ Packagizer: ordered rule chain (move/rename/destination; regex/extensions/size; `<name>`,`<ext>`,`<index>`,`<min/max-size>`, stop-after-rule) with the JD2 variable set (`<packagename>`, `<simpledate:…>`) and capture groups `$1..$9`,`${name}`
- ✅ Bottom docks both tabs: Download Overview + File Properties (downloads); Linkgrabber Overview + Package Properties (grabber)
- ✅ Status strip: count/speed/debrid premium chips
- 🔄 Property editors in the grabber package box (name, folder, password, extract/delete — present; parity polish pending)
- 📋 Linkgrabber context menu JD2-flavored; column filter funnels in header (JD2's filter drop-downs)
- 📋 JD2-style caption (title bar: icon + title center, flat buttons) instead of the Breeze-style one
- 📋 Speed meter as a real line graph (JD2's meter), not just text

## Engine & download workflow

- ✅ Real engine (upstream): segmented HTTP/1.1, resume, per-host limits, speed limiter, checksums, FTP/FTPS, mirrors
- ✅ aria2 idea set aside — engine kept native
- ✅ End-to-end happy path verified: `http://127.0.0.1:8123/notes.pdf` → 6 MB → correct category dir, correct bytes
- 📋 Multi-mirror list handling UI (JD2 shows "all mirrors" per download)
- 📋 Reconnect (router IP change) — low demand; scoped out until a user asks (options → Reconnect analog absent)

## Packagizer roadmap

- ✅ Rules engine, dialog, persistence (`settings.packagizer_rules`)
- ✅ Apply-at-add-time in the grabber + "Apply to list"
- 📋 Priority action (needs engine support: ordering by priority)
- 📋 Destination regex variable groups across rules (e.g., move renamed archive to subfolder named by group)
- 📋 Built-in starter rule packs (archives → `Archives`, videos → `Videos`, …) as templates in the dialog

## Trust & safety

- ✅ Packagizer regexes are inert-on-compile-fail; no panic paths; rename never produces empty name
- ✅ ".." refused in destination payloads
- 🔄 Credential handling: debrid/site logins already `keychain:`-backed; audit AD-hoc logging for leaks of cookies/passwords in UI/log outputs
- 📋 Plugin sandbox (in-process, untrusted) — if and when a plugin system lands, it must not be JVM-in-process like JD2's
- 📋 SubConfiguration-style obfuscation deliberately not carried forward

## Packaging & release

- ✅ `tools/installer/build-linux.sh` in place (script), not yet run locally (no `pwsh`/`rpmbuild`/`dpkg-deb` on this host without sudo)
- 📋 `.deb` / `.rpm` / AppImage built by CI, not by hand (GitHub Actions)
- 📋 Nightlies (`nightly` branch) built automatically
- 📋 Snap/Flatpak community packaging
- 🔄 Fork prebuilt binaries via GitHub releases (once CI green)

## Upstream maintenance

- 📋 Periodic upstream merge (every minor release): verify fork keeps compiling; fork-only files: `crates/app/assets/themes/jdownloader*`, `crates/app/assets/themes/jdownloader/icons/`, fork-added modules (`packagizer.rs`, `dock.rs`), `docs/fork-roadmap.md`, `NOTICE.md`
- 📋 Mirror test: every upstream test still passes (fork commits `g` must not break existing tests)

## Guardrails

- Never copy JD2 binary/assets/strings for the look; analysis may inform structure only (see `NOTICE.md`)
- Never copy MDM GPL code into permissively-licensed projects we ship (none currently)
- Upstream GPL-3.0 compliance: `LICENSE` remains GPL-3.0, attribution kept
