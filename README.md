# zDownloader³

<p align="center"><img src="website/assets/banner.svg" alt="zDownloader³" width="100%" /></p>

> **Alpha builds are for testing only.** This is a pre-release fork of upstream's
> Mister Download Manager. Expect crashes, missing pieces and rough edges. **Do not trust it
> for downloads you cannot afford to lose.** Bug reports on the [issue tracker](https://github.com/nihil-vt/ZDownloader3/issues) welcome
> — attach the `zdownloader3 --help` output and `logs/mdm.log` when filing one.

A native Linux-first download manager with a JDownloader2-shaped UI: a category sidebar, a
`Downloads`/`Linkgrabber` tab pair, a package-grouped link list, and a flat, fast-darkmed
theme. Built in Rust on egui, with an engine that splits across connections, resumes, and
verifies checksums.

## Features

- **JD2-style link grabber**: paste or drop links (or let the browser add them) → packages
  auto-group by folder or archive volumes, tick what you want, **Add to queue** / **Start**
- **Packagizer**: a rule chain that moves, renames and re-destinates links by URL/name
  regex and extension rules, at add time
- **A JD2-look theme** (`View → Theme → JDownloader2`), full light/dark, with dense flat
  table styling and JD2-tinted toolbar glyphs
- **Dock panels**: Download Overview and File Properties on the `Downloads` tab;
  Linkgrabber Overview and Package Properties on the grabber tab
- **Tray + silent mode**, close-to-tray, show/hide hotkey, sounds, starter of downloaded
  archives, Click'n'Load with per-site consent, browser extension for Firefox and Chromium
- **Headless / scriptable** — `zdownloader3-cli get` downloads without the window; the automation
  API (`api on` → `status`/`pause`/`start`/...) exposes the running app to scripts
- **Torrents** (magnets + files), **debrid account** routing, extraction of zip / 7z / rar
  (when 7-Zip is installed)

## Build and run

```sh
cargo run -p mdm-app           # debug
cargo install --path crates/app  # install
```

`cargo` and a C compiler are the only requirements; `tools/installer/build-linux.sh`
packages `.deb` / `.rpm` / AppImage (see its header for tooling).

## Documentation

- `docs/README.md` — the user guide (most pages describe what upstream ships; fork differences are noted there)
- `website/` — the fork's website (`website/index.html`; `python website/build.py` re-renders the roadmap/changelog pages)
- `docs/fork-roadmap.md` — fork plans and status
- `CONTRIBUTING.md` — how to send patches

## License

GNU GPL-3.0. zDownloader³ descends from Mister Download Manager by Ahmed Nefzaoui; see
`NOTICE.md` for provenance and `LICENSE` for the terms. Attribution is kindly retained.
Not affiliated with jdownloader.org, Eye/appwork, or misterdownloadmanager.com.
