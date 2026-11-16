# JD2 → MDM gap plan — what the fork still needs from JDownloader2

MDM already covers or exceeds JD2 for: engine speed, list/package UX, linkgrabber-style triage (look-ups, dedup, archive grouping, CNL2-with-allowlist), extraction (zip/7z/rar via 7-Zip/hatchzip), queues+scheduler, tray/remote/automation API+MCP, torrents, updates, Linux packaging (.deb/.rpm/AppImage), keychained secrets, crash reports, i18n (86 languages).

JD2 still has, MDM does not:

## 1. Host/crawler plugin engine — the entire category (highest effort, or zero)
JD2 ships ~1,000 Java plugins, loaded from an update channel, in-process, insecure.
MDM has built-in site modules only (gdrive, mediafire, plus debrid services).

Decision needed before any code: **do we build a native plugin engine at all?**
- Option 1 (recommended start): keep MDM's in-tree site modules pattern (`crates/engine/src/sites/`) and add the top hosts/sites JD2 users ask for, one PR each. No plugin API.
- Option 2: design `SitePlugin` trait + manifest + signed plugins + out-of-process execution (reference: JD2 PluginForHost/PluginForDecrypt lifecycle + captcha challenge chain). A product decision, weeks-to-months.
- Option 3 (reject): run JD2 Java plugins via JVM bridge — wrong ecosystem, license traps.

Until Option 2 is explicitly scoped, nothing here blocks the other gaps.

## 2. Packagizer-style user rules (medium)
JD2: ordered rules over crawled links — rename (`<jd:packagename>`), filetype/hoster/regex conditions, move-to-package, priority, autostart/autoextract, stop-after-rule. MDM: fixed archive-volume grouping + dedup only.
→ Add `RulesDialog` + engine hook: condition list (regex/filetype/size/hoster) × action (rename pattern, destination subfolder, enable/disable, priority), JSON-persisted, applied at add/look-up time in linkgrabber. ~2–4 weeks including UI.

## 3. Premium hoster accounts (large, gated by plugin decision)
JD2: per-host premium login + traffic accounting (Rapidgator, keep2share…). MDM: debrid only.
→ Same shape as debrid module but per-host: account store, traffic check before enqueue, cookie session reuse. Needs a few target hosts chosen first; do not build generic before picking 2–3.

## 4. Captcha solving chain (large, gated by 2/3)
JD2: manual dialog + JAC/9kw/DBC/twocaptcha services + quality feedback. MDM: none (no captcha hosts).
→ Only meaningful with Option 2 plugins. Manual-solve dialog is the first step regardless.

## 5. MyJDownloader-class cloud remote — mostly closed (small)
MDM's automation API + HTTPS remote (pairing codes, device tokens, LAN) + MCP server already do this better; JD2's MyJD remains useful only for its internet-facing relay. Skip unless users ask; document API as our answer.

## 6. Router reconnect (small-medium, low demand)
JD2: live-header scripts + UPnP for FritzBox. MDM: none (connection shedding ≠ IP change).
→ Optional `reconnect` module: HTTP replay scripts + UPnP, config-gated, off by default. Only if a user segment asks; DSL/fiber/CGNAT makes it rare in 2026.

## 7. What MDM does better — keep, don't revert
No jdcheck answers, per-origin CNL2 consent, look-up etiquette, keychain refs, atomic writes, lab-server fault injection, crash minidumps, wake-on-RTC. Our differentiator vs JD2.

## Suggested order (post fork-import)
1. Ship rename + Adwaita default + verify build green in CI (in progress)
2. Gap 2: packagizer rules — self-contained, immediately JD2-visible
3. Gap 1 decision, then Option 1 site modules for top JD2 hosts (user research)
4. Gap 3/4 only after a plugin-shaped use case exists
5. Gap 5/6 documentation or backlog

Estimates assume one developer familiar with the tree. Update this file as decisions land (it tracks against ROADMAP.md, which is upstream's and stale for the fork).
