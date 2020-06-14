# gDiceRoller
## A multifunction polyhedral dice simulator for GNOME

gDiceRoller uses the RFYL library to roll arbitrary collections of dice and do arbitrary
arithmetic on their results in real time.

![A screenshot of gDiceRoller](data/screenshot.png)

## Building

Building gDiceRoller requires `rustc`, `cargo`, and GTK3 development libraries.
All of gDiceRoller's Rust dependencies are vendored in the `vendor` directory, meaning
that building does not require an active internet connection once the repository has
been cloned.

gDiceRoller uses GNU Make. Simply:

```bash
make
sudo make install
# to remove
sudo make uninstall
```

To build with Flatpak, you need the `flatpak-builder` and the following dependencies:

```bash
# Add flathub and the gnome-nightly repo
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak remote-add --user --if-not-exists gnome-nightly https://sdk.gnome.org/gnome-nightly.flatpakrepo

# Install the gnome-nightly Sdk and Platform runtime
flatpak install --user gnome-nightly org.gnome.Sdk org.gnome.Platform

# Install the required rust-stable extension from flathub
flatpak install --user flathub org.freedesktop.Sdk.Extension.rust-stable//18.08
```

Then, `make flatpak`. The package is built into `flatpak/` so you can run
`flatpak-builder --run flatpak/ data/codes.nora.gdiceroller.json codes.nora.gdiceroller`
or publish to a repo.

## TODO

- [ ]  Internationalization
- [ ]  Code tidying
- [x]  Icons and app metadata
- [x]  Flatpak
- [ ]  Snap package
- [ ]  .deb and .rpm packages

