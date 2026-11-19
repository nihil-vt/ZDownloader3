#!/usr/bin/env bash
# UI iteration helper: rebuild and relaunch zDownloader3 whenever a source file changes.
# Uses a scratch ZD3_DATA_DIR so your real list is untouched.
# No cargo-watch, no inotify-tools — 1-second mtime polling, setsid relaunch, pkill the running instance.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

SCRATCH="${ZD3_WATCH_DIR:-/tmp/zd3-watch-data}"
BIN="$ROOT/target/debug/zdownloader3"
mkdir -p "$SCRATCH"

# Watched set: Rust sources, Cargo manifests, icons, catalogs, widget config.
last_seen() {
  find crates i18n tools -type f \( -name '*.rs' -o -name '*.toml' -o -name '*.svg' -o -name '*.tsv' -o -name '*.png' -o -name '*.json' \) -printf '%T@\n' 2>/dev/null | sort -n | tail -1
}

relaunch() {
  pkill -x zdownloader3 2>/dev/null || true
  ZD3_DATA_DIR="$SCRATCH" setsid "$BIN" >/dev/null 2>&1 </dev/null &
}

LAST=""
FIRST=1
while true; do
  cur=$(last_seen)
  if [ "$cur" != "$LAST" ] || [ "$FIRST" -eq 1 ]; then
    echo "[watch] building..."
    if cargo build -p mdm-app 2>/tmp/opencode/watch-build.log; then
      relaunch
      FIRST=0
    else
      echo "[watch] build failed — fix errors, next save retries"
    fi
    LAST=$cur
  fi
  sleep 1
done
