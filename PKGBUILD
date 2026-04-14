# Maintainer: jakes <jakes@users.noreply.github.com>
pkgname=papery
pkgver=0.1.0
pkgrel=1
pkgdesc="Native COSMIC desktop wallpaper changer - like Variety but built with Rust and libcosmic"
arch=('x86_64')
url="https://github.com/jibsta210/papery"
license=('GPL-3.0-only')
depends=('cosmic-bg' 'dbus')
makedepends=('cargo' 'just' 'cmake' 'pkgconf')
source=("$pkgname-$pkgver::git+https://github.com/jibsta210/papery.git#tag=v$pkgver")
sha256sums=('SKIP')

prepare() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 "target/release/papery" "$pkgdir/usr/bin/papery"
    install -Dm644 "data/dev.papery.CosmicApplet.desktop" "$pkgdir/usr/share/applications/dev.papery.CosmicApplet.desktop"
    install -Dm644 "data/dev.papery.CosmicApplet.metainfo.xml" "$pkgdir/usr/share/metainfo/dev.papery.CosmicApplet.metainfo.xml"
    install -Dm644 "data/icons/scalable/apps/dev.papery.CosmicApplet.svg" "$pkgdir/usr/share/icons/hicolor/scalable/apps/dev.papery.CosmicApplet.svg"
    install -Dm644 "data/dev.papery.CosmicApplet-autostart.desktop" "$pkgdir/etc/xdg/autostart/dev.papery.CosmicApplet.desktop"
    install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
