# JD2 look reference (tokens for the `jdownloader` theme)

Captured 2026-11-16 from the shipped JD2 (flatpak, rev 50639) + FlatLaf conventions. Analysis only — no JD2 assets are copied. Icons here are described by role, not reproduced.

## Window skeleton (what the skin must evoke)

- **Title bar** FlatLaf light: light grey chrome, centered title + app icon. (Our header keeps MDM's decorations; `caption` roles carry the grey.)
- **Menu bar**: classic flat menu (`File | Downloads | Help`), light grey, hover = flat blue-grey button.
- **Toolbar**: flat, icon+text optional, icon-first; separator lines; starts at left.
- **Tabs**: `Linkgrabber | Downloads` (+ Settings/MyJD closable) — flat tab headers with bottom accent line on the active tab.
- **Main area**: package table, white surface, very dense rows (~22 px), zebra off by default, package rows tinted light blue-grey when "packages background highlight" is on, children indented.
- **Status bar**: thin strip, segmented: premium chips left, status text center-left, speed meter right; icon row right-aligned.
- **Scrollbars**: narrow (14 px), light.

## Palette (FlatLaf light, observed + conventional)

| Token | Value | Notes |
|---|---|---|
| `background` (window) | `#eceff3` | FlatLaf light panel |
| `surface` (table/list) | `#ffffff` | |
| `header` (toolbar/menu/caption) | `#e6e8eb` / `#eceff3` | single grey ramp |
| `text` | `#24292f` | |
| `text-dim` | `#6b7280` | |
| `accent` | `#3d6db5` | JD2 toolbar blue (start button is green `#4f9c06`; download row accent is blue) |
| `accent-strong` | `#2f5580` | pressed |
| `selection` | `#c9dcef` | FlatLaf-ish soft blue; dark text on it |
| `selection-active` | `#3d6db5` | white text when window focused? FlatLaf keeps soft blue |
| `success` (start) | `#4f9c06` | JD2 start button green |
| `warning` | `#c47b12` | |
| `error` | `#c4351c` | |
| `border` | `#c8ccd1` | |
| `row-alt` | none | zebra off by default in JD2 |
| `package-row` | `#e9edf3` | package background highlight on |

## Palette (FlatLaf dark, for the dark variant)

| Token | Value |
|---|---|
| `background` | `#2e3136` |
| `surface` | `#24272b` |
| `header` | `#33363b` |
| `text` | `#dde1e6` |
| `accent` | `#6aa3e0` |
| `selection` | `#2f4a6b` |
| `package-row` | `#2a3138` |

## Density / shape

| Metric | Value |
|---|---|
| Table row height | 22 px (our `radius.egui` 0–2, compact preset) |
| Radius | 2 px controls, 0 table cells |
| Scrollbar | 12–14 px |
| Font | system UI font at 100% interface size; JD2 uses ~12–13 pt |
| Accent line | 2 px under active tab, same accent |

## Icon style (redrawn, not copied)

Flat glyphs, 2-tone: single-color silhouettes in `#3d6db5` (accent) or `#4f9c06` (action green) with a faint white inner highlight for toolbar glyphs — or pure mono silhouettes for the symbolic set. Toolbar: 32 px with 16/24/32/48 png sizes generated at build time.

## Non-goals / things to NOT reproduce

- JD2's glossy color PNGs (we draw flat equivalents)
- Donate/promo tabs, banner ad space
- `jdcheck.js`-style program detection
- Swing focus rectangles (egui ring is the analogue)
