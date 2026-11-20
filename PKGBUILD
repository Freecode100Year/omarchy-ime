pkgname=omarchy-ime-git
pkgver=0.1.0.r0.g0000000
pkgrel=1
pkgdesc='Offline Rust Chinese input engine for Fcitx5 on Omarchy/Hyprland'
arch=('x86_64' 'aarch64')
url='https://github.com/Freecode100Year/omarchy-ime'
license=('MIT' 'Apache-2.0' 'GPL-3.0-only')
depends=('fcitx5' 'python')
optdepends=('fcitx5-gtk: GTK toolkit integration' 'fcitx5-qt: Qt toolkit integration' 'fcitx5-configtool: configure input methods and UI')
makedepends=('git' 'rust' 'cmake' 'gcc' 'fcitx5')
source=()
sha256sums=()
pkgver() { cd "$startdir"; printf '0.1.0.r%s.g%s' "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"; }
build() {
  cd "$startdir"
  cargo build --release -p omarchy-ime
  cmake -S frontend/fcitx5 -B build/fcitx5 -DCMAKE_BUILD_TYPE=Release
  cmake --build build/fcitx5
}
package() {
  cd "$startdir"
  install -Dm755 target/release/libomarchy_ime.so "$pkgdir/usr/lib/fcitx5/libomarchy_ime.so"
  DESTDIR="$pkgdir" cmake --install build/fcitx5 --prefix /usr
  install -Dm755 target/release/omarchy-ime "$pkgdir/usr/bin/omarchy-ime"
  install -Dm755 theme-sync.py "$pkgdir/usr/bin/omarchy-ime-theme-sync"
  install -Dm644 data/rime-ice-pinyin.tsv "$pkgdir/usr/share/omarchy-ime/pinyin.tsv"
  install -Dm644 data/source-8105.dict.yaml "$pkgdir/usr/share/doc/omarchy-ime/source-8105.dict.yaml"
  install -Dm644 data/source-base.dict.yaml "$pkgdir/usr/share/doc/omarchy-ime/source-base.dict.yaml"
  install -Dm644 config/omarchy-ime.toml "$pkgdir/usr/share/omarchy-ime/config.toml"
  install -Dm644 data/THIRD_PARTY_NOTICES.md "$pkgdir/usr/share/doc/omarchy-ime/THIRD_PARTY_NOTICES.md"
  install -Dm644 data/licenses/GPL-3.0-rime-ice.txt "$pkgdir/usr/share/licenses/omarchy-ime-rime-ice/GPL-3.0.txt"
  install -Dm644 README.md "$pkgdir/usr/share/doc/omarchy-ime/README.md"
  install -Dm644 LICENSE-MIT "$pkgdir/usr/share/licenses/omarchy-ime/LICENSE-MIT"
  install -Dm644 LICENSE-APACHE "$pkgdir/usr/share/licenses/omarchy-ime/LICENSE-APACHE"
  install -Dm644 data/licenses/GPL-3.0-rime-ice.txt "$pkgdir/usr/share/licenses/omarchy-ime/GPL-3.0-rime-ice.txt"
}
