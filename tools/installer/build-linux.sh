#!/usr/bin/env bash
# Builds the Linux release for this machine's processor into target/dist/ (up to 0.2.4, each
# name ended in "-experimental"), <arch> being x86_64 on a 64-bit Intel or AMD machine and arm64
# on a 64-bit ARM one:
#   zDownloader3-<version>-linux-<arch>.AppImage        runs on most distros
#   zDownloader3-<version>-linux-<arch>.AppImage.zsync  its updates (AppImage tools)
#   zDownloader3-<version>-linux-<arch>.deb             Debian, Ubuntu, Mint
#   zDownloader3-<version>-symbols-linux-<arch>.zip     its debug symbols (for crash reports)
# Update now (crates/tools/src/update.rs), the website (tools/website/build.py) and the docs
# look for these names, and still for the "-experimental" ones of older releases.
# Needs: cargo, pwsh (the extension), python3 with Pillow (the icon), dpkg-deb, network access (appimagetool is fetched),
# and zsyncmake if the appimagetool fetched doesn't write the .zsync itself.
# Build on the oldest Ubuntu you want to support: the programs need its glibc or newer.
#   tools/installer/build-linux.sh [--no-build]
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
dist="$root/target/dist"
bin="$root/target/release"
version="$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' "$root/Cargo.toml" | head -1)"
[ -n "$version" ] || { echo "no version in Cargo.toml" >&2; exit 1; }
# The processor as the files' names say it (arch), and as Debian (deb_arch), RPM and AppImage
# (pkg_arch) do.
case "$(uname -m)" in
  x86_64) arch=x86_64 deb_arch=amd64 pkg_arch=x86_64 ;;
  aarch64) arch=arm64 deb_arch=arm64 pkg_arch=aarch64 ;;
  *) echo "no Linux release is built for $(uname -m)" >&2; exit 1 ;;
esac
name="zDownloader3-$version-linux-$arch"
programs=(zdownloader3 zdownloader3-nmhost zdownloader3-cli zdownloader3-mcp)

if [ "${1:-}" != "--no-build" ]; then
  # The build machine's paths out of the programs (#25): panic locations and debug info say
  # "cargo/..." for cargo's folder and "mdm/..." for this checkout. Encoded flags (unit separators),
  # as those paths may hold spaces.
  CARGO_ENCODED_RUSTFLAGS="--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=cargo"$'\x1f'"--remap-path-prefix=$root=mdm" \
    cargo build --release -p mdm-app -p mdm-nmhost -p mdm-cli -p mdm-mcp
fi
pwsh -NoProfile -File "$root/tools/extension/pack.ps1"

mkdir -p "$dist"
stage="$dist/stage-linux"
rm -rf "$stage"
mkdir -p "$stage"

# One tree in the usual Linux layout; every package is made from it.
tree="$stage/tree"
mkdir -p "$tree/usr/bin" "$tree/usr/share/applications" \
  "$tree/usr/share/icons/hicolor/256x256/apps" "$tree/usr/share/zdownloader3" \
  "$tree/usr/share/doc/zdownloader3"
for p in "${programs[@]}"; do install -m 755 "$bin/$p" "$tree/usr/bin/$p"; done
# Crash reports: the release's symbols from the programs' debug info (the -symbols zip), then the
# programs without it or their symbol table (#25; the build id that matches them to the symbols,
# and the unwind tables, stay).
python3 "$root/tools/installer/symbols.py" "linux-$arch" "$version" "$dist" "${programs[@]/#/$bin/}"
for p in "${programs[@]}"; do strip --strip-all "$tree/usr/bin/$p"; done
# The oldest glibc the programs run on: the newest GLIBC_x.y symbol version they ask for. The
# .deb requires it, so a system too old refuses the package instead of installing
# programs that can't start ("GLIBC_2.38 not found" on Debian 12 from a build on Ubuntu 24.04).
glibc=$(for p in "${programs[@]}"; do objdump -T "$tree/usr/bin/$p"; done \
  | grep -o 'GLIBC_[0-9][0-9.]*' | sed 's/GLIBC_//' | sort -V | tail -1)
[ -n "$glibc" ] || { echo "no glibc version found in the programs" >&2; exit 1; }
echo "the programs need glibc $glibc or newer"
cp -r "$root/target/extension/chromium" "$tree/usr/share/zdownloader3/extension-chromium"
cp "$root/target/extension/mdm-firefox.xpi" "$tree/usr/share/zdownloader3/"
cp "$root/LICENSE" "$root/ATTRIBUTION.md" "$tree/usr/share/doc/zdownloader3/"
python3 - "$root/art/zDownloader³.png" \
  "$tree/usr/share/icons/hicolor/256x256/apps/zdownloader3.png" <<'EOF'
import sys
from PIL import Image
Image.open(sys.argv[1]).convert("RGBA").resize((256, 256), Image.LANCZOS).save(sys.argv[2])
EOF
# StartupWMClass: the class every window of MDM carries (main.rs APP_ID), so a window pinned to
# the taskbar or the dock is this entry's, whatever name an AppImage manager gives the entry (#940).
cat > "$tree/usr/share/applications/zdownloader3.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=zDownloader³
Comment=Download manager
Exec=zdownloader3 %U
Icon=zdownloader3
Terminal=false
Categories=Network;FileTransfer;
StartupWMClass=zdownloader3
MimeType=x-scheme-handler/magnet;application/x-bittorrent;
EOF

# .deb. Its dependencies: the libraries the window loads at run time. Without
# libxkbcommon-x11 (winit's keyboard under X11, and XWayland, MDM's default in a Wayland session)
# the window panics at start; EGL draws Wayland windows; without the CA certificates no download
# starts, HTTP ones too ("No CA certificates were loaded from the system"). GTK pulls in the rest.
# The browsers' host manifests for everyone (#266), in the .deb only: each folder the
# browsers read them from (/etc/opt/chrome, /etc/chromium, /etc/opt/edge, /usr/lib/mozilla and
# /usr/lib64/mozilla), naming /usr/bin/zdownloader3-nmhost. The package owns them, so
# uninstalling it removes them (not conffiles: `apt remove` takes them too), and MDM writes none
# in the home folder for a browser that reads them. The AppImage registers per user.
system="$stage/system"
"$tree/usr/bin/zdownloader3-nmhost" --package-manifests "$system"
deb="$stage/deb"
cp -r "$tree" "$deb"
# Debian and Ubuntu's Firefox reads /usr/lib/mozilla; lib64's is Fedora's and openSUSE's.
cp -r "$system/." "$deb/"
rm -rf "$deb/usr/lib64"
mkdir -p "$deb/DEBIAN"
cat > "$deb/DEBIAN/control" <<EOF
Package: zdownloader3
Version: $version
Architecture: $deb_arch
Maintainer: ZDownloader3 <https://github.com/nihil-vt/ZDownloader3>
Depends: libc6 (>= $glibc), libgtk-3-0, libgl1, libegl1, libxkbcommon0, libxkbcommon-x11-0, ca-certificates
Section: net
Priority: optional
Homepage: https://nihil-vt.github.io/ZDownloader3/
Description: Download manager
 Speeds up downloads with several connections at once, resumes them, catches them from the
 browser, and downloads videos and torrents.
EOF
dpkg-deb --root-owner-group --build "$deb" "$dist/$name.deb"

# AppImage: the same tree as an AppDir.
appdir="$stage/AppDir"
cp -r "$tree" "$appdir"
ln -s usr/bin/zdownloader3 "$appdir/AppRun"
cp "$tree/usr/share/applications/zdownloader3.desktop" "$appdir/"
cp "$tree/usr/share/icons/hicolor/256x256/apps/zdownloader3.png" "$appdir/"
tool="$stage/appimagetool"
python3 "$root/tools/installer/fetch_release.py" AppImage/appimagetool continuous "appimagetool-$pkg_arch.AppImage" "$tool"
chmod +x "$tool"
# Update information (#24): AppImageLauncher, Gear Lever and AppImageUpdate read it from the
# AppImage, find the latest release's .zsync and download only the parts that changed. MDM's own
# Update now doesn't need it. The pattern also matches the names up to 0.2.4, with "-experimental".
# The file name glob stays all-ASCII: fnmatch's `*` matches the superscript in the release
# asset names anyway, and appimagetool's option parser chokes on the raw ³ bytes.
update="gh-releases-zsync|nihil-vt|ZDownloader3|latest|zDownloader*-linux-$arch*.AppImage.zsync"
# No FUSE on CI runners: the tool runs from its own extracted copy.
# zstd at its strongest level (#25; unpacking is as fast): the tool's 128K blocks stay, as 1M
# blocks make the AppImage start slower (AppImage/appimagetool#64).
# appimagetool's option parser takes its argument bytes as Latin-1 and fails on the superscript
# ("Option parsing failed"), no matter where they appear in argv: build under an all-ASCII name
# and rename the finished files afterwards. The release asset names keep the superscript.
ascii_name="zDownloader3-$version-linux-$arch"
if ! APPIMAGE_EXTRACT_AND_RUN=1 ARCH=$pkg_arch "$tool" -u "$update" \
  --mksquashfs-opt -Xcompression-level --mksquashfs-opt 22 "$appdir" "$dist/$ascii_name.AppImage"; then
  echo "appimagetool rejected the update information; building without it" >&2
  APPIMAGE_EXTRACT_AND_RUN=1 ARCH=$pkg_arch "$tool" \
    --mksquashfs-opt -Xcompression-level --mksquashfs-opt 22 "$appdir" "$dist/$ascii_name.AppImage"
fi
# appimagetool writes the .zsync itself (beside the AppImage, or in the current folder); with
# one that doesn't, zsyncmake makes it.
[ -f "$ascii_name.AppImage.zsync" ] && mv "$ascii_name.AppImage.zsync" "$dist/"
if [ ! -f "$dist/$ascii_name.AppImage.zsync" ]; then
  (cd "$dist" && zsyncmake -u "$ascii_name.AppImage" -o "$ascii_name.AppImage.zsync" "$ascii_name.AppImage")
fi
mv "$dist/$ascii_name.AppImage" "$dist/$name.AppImage"
mv "$dist/$ascii_name.AppImage.zsync" "$dist/$name.AppImage.zsync" 2>/dev/null || true

rm -rf "$stage"
(cd "$dist" && sha256sum "$name".* > SHA256SUMS-linux.txt)
ls -l "$dist"
