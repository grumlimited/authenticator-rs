AUTHENTICATOR-RS
==================
![Continuous integration](https://github.com/grumlimited/authenticator-rs/workflows/Continuous%20integration/badge.svg?branch=master)

Authenticator-rs is a TOTP-MFA application written in Rust and GTK3.

This application is very much a work in progress.

It is initially inspired by [authenticator](https://gitlab.gnome.org/World/Authenticator), which sadly sort of 
[broke](https://aur.archlinux.org/packages/authenticator/) for me 
in the latest versions of python shipped with [Arch Linux](https://www.archlinux.org/).

It is by no means as feature-rich as its python relative, more like a diamond in the rough. Well, maybe not a diamond, 
but definitely in the rough...

![authenticator-rs](./authenticator-rs-main.png "Main view")
![authenticator-rs](./authenticator-rs-edit-account.png "Main view")
![authenticator-rs](./authenticator-rs-add-group.png "Main view")

## License

Authenticator-rs is published under the [GNU GENERAL PUBLIC LICENSE v3](./README.md).

## Changelog

See [releases](https://github.com/grumlimited/authenticator-rs/releases).

## Installing

Download from the [release](https://github.com/grumlimited/authenticator-rs/releases) page.

#### Debian

    dpkg -i authenticator-rs-x.x.x-x86-64.deb

#### Arch Linux

Authenticator RS is also available from [AUR](https://aur.archlinux.org/packages/authenticator-rs-bin/). Or from the release page:

    pacman -U authenticator-rs-bin-x.x.x-1-x86_64.pkg.tar.xz
    

## Building (dev)

    make target/release/authenticator-rs
    
    ./target/release/authenticator-rs
    
## Running (dev)

    make run
    
## Assets

Icon files are from [authenticator](https://gitlab.gnome.org/World/Authenticator).

Original GTK template from [Nora Codes - gDiceRoller](https://nora.codes/tutorial/speedy-desktop-apps-with-gtk-and-rust/).
