# Maintainer: Ar Rifat <arrifat346afs@users.noreply.github.com>

pkgname=veyra
pkgver=0.1.1
pkgrel=1
pkgdesc="A file manager for Linux built with GPUI"
arch=('x86_64')
url="https://github.com/arrifat346afs/Veyra"
license=('unknown') # the repo ships no LICENSE file yet
depends=('glib2' 'gcc-libs' 'glibc' 'mesa' 'wayland' 'libxkbcommon' 'libxkbcommon-x11' 'libx11' 'libxcb' 'fontconfig' 'freetype2')
optdepends=(
  'papirus-icon-theme: file and folder icons'
  'poppler: PDF previews'
  'ffmpegthumbnailer: video thumbnails'
  'libreoffice-fresh: Office document previews'
)
makedepends=('cargo' 'clang' 'cmake' 'pkgconf')
options=('!lto')
# Builds from the checkout this PKGBUILD sits in; no remote sources.
source=()
sha256sums=()

build() {
  cargo build --release --locked --manifest-path "$startdir/Cargo.toml"
}

package() {
  install -Dm755 "$startdir/target/release/veyra" "$pkgdir/usr/bin/veyra"
  install -Dm644 "$startdir/integration/veyra.desktop" "$pkgdir/usr/share/applications/veyra.desktop"
  for size in 16 24 32 48 64 128 256 512; do
    install -Dm644 "$startdir/icons/hicolor/${size}x${size}/apps/veyra.png" \
      "$pkgdir/usr/share/icons/hicolor/${size}x${size}/apps/veyra.png"
  done
  install -Dm644 "$startdir/integration/veyra.portal" \
    "$pkgdir/usr/share/xdg-desktop-portal/portals/veyra.portal"
  sed 's|@BINARY@|/usr/bin/veyra|' \
    "$startdir/integration/org.freedesktop.impl.portal.desktop.veyra.service.in" \
    > "$srcdir/org.freedesktop.impl.portal.desktop.veyra.service"
  install -Dm644 "$srcdir/org.freedesktop.impl.portal.desktop.veyra.service" \
    "$pkgdir/usr/share/dbus-1/services/org.freedesktop.impl.portal.desktop.veyra.service"
}
