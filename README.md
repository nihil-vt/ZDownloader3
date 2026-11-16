# ZDownloader3

A native Linux-first download manager with a JDownloader2-shaped UI, forked from
**Mister Download Manager** (GPL-3.0, upstream `main` @ `1d151be`, 2026-11-16; see `NOTICE.md`).

<p align="center"><img src="docs/assets/brand/banner.svg" alt="ZDownloader3" width="100%" /></p>

## What works in the fork so far

- JDownloader2-like UI: `Downloads` / `Linkgrabber` tabs, package-grouped list, flat JD2-style theme
  (`View → Theme → JDownloader2`, the default on Linux)
- **Linkgrabber**: packages grouped by folder (same folder leaf, same file class, mirrors included) and
  by archive volumes (host ignored); `Name | Hoster | Size | Status` columns; `Check/Uncheck all`,
  look-up status, Add to queue / Start
- **Packagizer** (Downloads → Packagize&r): JDownloader2's rule chain (URL/name regexes, extension, size
  bounds, `<name>`, `<index>`, `<simpledate:…>`, `$1…`, `${name}`) moving links between packages,
  renaming them, or retargeting destinations
- **Dock panels** on both tabs (`View →`): Download Overview / File Properties; Linkgrabber Overview /
  Package Properties
- Tray icon, silent mode, close-to-tray, hotkey to show/hide, start-on-login, sounds

## Build and run

```sh
cargo run -p mdm-app          # or: cargo install --path crates/app
```

Linux packages (.deb, .rpm, AppImage) are built by `tools/installer/build-linux.sh`
(needs `pwsh` to pack the extension; see the script header for `rpmbuild`, `dpkg-deb`).

## Read

- `NOTICE.md` — provenance and license notes
- `docs/README.md` — user guide
- `docs/fork-roadmap.md` — fork roadmap and status
- `docs/gap-plan.md` — JD2 → MDM gap analysis and what the fork will / won't fix
- Upstream changes migrate into the fork periodically; upstream docs apply as written.
- `LICENSE` — GNU GPL-3.0 (this fork keeps it; attribution retained)

## What ZDownloader3 is not

- Not affiliated with JDownloader, Eye/appwork, or misterdownloadmanager.com.
- Not an interoperable reimplementation of the closed-source JDownloader2 — no JD2 code, icons or
  strings are copied or distributed here.
