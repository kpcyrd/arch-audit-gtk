# arch-audit-gtk

Show an indicator if there are any security updates missing for your Arch Linux
system.

![screenshot](docs/arch-audit-gtk.png)

## Install

    git clone https://aur.archlinux.org/arch-audit-gtk.git
    cd arch-audit-gtk
    makepkg -si

## Gnome3

On gnome3 you need to install and enable an
[extension](https://extensions.gnome.org/extension/615/appindicator-support/)
for AppIndicator to work.

## Development

    pacman -S git rust arch-audit
    git clone https://github.com/kpcyrd/arch-audit-gtk
    cd arch-audit-gtk
    cargo run

## License

GPLv3+
