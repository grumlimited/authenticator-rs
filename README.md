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

<kbd>![authenticator-rs](./authenticator-rs-main.png "Main view")</kbd>
<kbd>![authenticator-rs](./authenticator-rs-edit-account.png "Main view")</kbd>
<kbd>![authenticator-rs](./authenticator-rs-add-group.png "Main view")</kbd>

## License

Authenticator-rs is published under the [GNU GENERAL PUBLIC LICENSE v3](./README.md).

## Changelog

See [releases](https://github.com/grumlimited/authenticator-rs/releases).

## Installing

#### Debian

Download from the [release](https://github.com/grumlimited/authenticator-rs/releases) page.

    dpkg -i authenticator-rs-0.0.8-x86-64.deb

## Building

    cargo build
    
    cargo install --path=.
    
    $HOME/.cargo/bin/authenticator-rust
    
## Assets

Icon files are from [authenticator](https://gitlab.gnome.org/World/Authenticator).

Original GTK template from [Nora Codes - gDiceRoller](https://nora.codes/tutorial/speedy-desktop-apps-with-gtk-and-rust/) .