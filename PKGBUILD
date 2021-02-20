# Maintainer: kpcyrd <kpcyrd[at]archlinux[dot]org>

pkgname=arch-audit-gtk
pkgver=0.0.0
pkgrel=1
pkgdesc='Arch Linux Security Update Notifications'
url='https://github.com/kpcyrd/arch-audit-gtk'
arch=('x86_64')
license=('GPL3')
depends=('arch-audit' 'libappindicator-gtk3')
makedepends=('cargo' 'clang' 'llvm')

build() {
  cd ..
  #cargo build --release --locked
  cargo build --locked
}

package() {
  cd ..
  #install -Dm 755 target/release/${pkgname} -t "${pkgdir}/usr/bin"
  install -Dm 755 target/debug/${pkgname} -t "${pkgdir}/usr/bin"
  install -Dm 644 icons/*.svg -t "${pkgdir}/usr/share/arch-audit-gtk/icons"
  install -Dm 644 contrib/arch-audit-gtk.tmpfiles "${pkgdir}/usr/lib/tmpfiles.d/arch-audit-gtk.conf"
  install -Dm 644 contrib/arch-audit-gtk.hook "${pkgdir}/usr/share/libalpm/hooks/arch-audit-gtk.hook"
  install -Dm 644 contrib/arch-audit-gtk.desktop -t "${pkgdir}/etc/xdg/autostart"
}

# vim: ts=2 sw=2 et:
