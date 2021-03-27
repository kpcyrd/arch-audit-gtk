# arch-audit-gtk

Show an indicator if there are any security updates missing for your Arch Linux
system.

![screenshot](docs/arch-audit-gtk.png)

## Install

    pacman -S arch-audit-gtk

## Gnome3

For gnome3 you need to install an extension for app indicator support:

    pacman -S gnome-shell-extension-appindicator

After installing this extension you need to restart your desktop so gnome picks it up, you then need to enable the extension with the gnome extension manager.

## Development

    pacman -S git rust arch-audit
    git clone https://github.com/kpcyrd/arch-audit-gtk
    cd arch-audit-gtk
    cargo run

## License

GPLv3+
