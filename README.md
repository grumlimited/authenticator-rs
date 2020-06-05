AUTHENTICATOR-RS
==================
Authenticator-rs TOTP MFA application written in Rust.

This application is very much a work in progress.

It is initially inspired from (authenticator)[https://gitlab.gnome.org/World/Authenticator], which sadly sort of 
[broke](https://aur.archlinux.org/packages/authenticator/) for me 
in the latest versions of python shipped with [Arch Linux](https://www.archlinux.org/) 

It is by no means as feature-rich as its python relative, more like a diamond in the rough. Well, maybe not a diamond, 
but definitely in the rough...

Lastly, it is using this fantastic library: [iced](https://github.com/hecrj/iced) as the building blocks for it UI.
Thanks @ecrj and the iced team.

<kbd>![authenticator-rs](./authenticator-rs.png "Authenticator RS")</kbd>

Authenticator-rs is published under the [GNU GENERAL PUBLIC LICENSE v3](./README.md).

## State of affairs

### What's working

* generating totp tokens for multiple accounts
* copy and pasting tokens to clipboard
* only tested on Linux (Arch Linux to be specific)

### What's missing

Well, pretty much everything else ;-)

* in-app ability to edit accounts
* multiple themes support
* modal dialogs for imports/exports

