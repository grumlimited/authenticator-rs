# Install to /usr unless otherwise specified, such as `make PREFIX=/app`
PREFIX=/usr

# What to run to install various files
INSTALL=install
# Run to install the actual binary
INSTALL_PROGRAM=$(INSTALL)
# Run to install application data, with differing permissions
INSTALL_DATA=$(INSTALL) -m 644

# Directories into which to install the various files
bindir=$(DESTDIR)$(PREFIX)/bin
sharedir=$(DESTDIR)$(PREFIX)/share

# These targets have no associated build files.
.PHONY : clean clean-all install uninstall

check-update: # cargo install cargo-update
	cargo install-update -a

# Build the application
release : src
	cargo build --release

# Compiling gResource
gresource:
	glib-compile-resources data/uk.co.grumlimited.authenticator-rs.xml

test:
	cargo test

run: gresource
	cargo run

clippy:
	cargo fmt
	find src/ -name "*.rs" -exec touch {} \;
	cargo clippy

release-version:
	sed -i 's/#VERSION_NUMBER#/$(RELEASE_VERSION)/' ./data/resources/gtk/ui/main.ui

install-po: # dev only - run with sudo
	msgfmt po/fr.po -o $(sharedir)/locale/fr/LC_MESSAGES/authenticator-rs.mo
	msgfmt po/en_GB.po -o $(sharedir)/locale/en_GB/LC_MESSAGES/authenticator-rs.mo

install-gresource: gresource
	pwd
	# Install gResource
	mkdir -p $(sharedir)/uk.co.grumlimited.authenticator-rs/
	$(INSTALL_DATA) data/uk.co.grumlimited.authenticator-rs.gresource $(sharedir)/uk.co.grumlimited.authenticator-rs/uk.co.grumlimited.authenticator-rs.gresource

#	# Install icons
#	mkdir -p $(sharedir)/icons/hicolor/scalable/apps/
#	$(INSTALL_DATA) data/icons/hicolor/scalable/apps/uk.co.grumlimited.authenticator-rs.svg $(sharedir)/icons/hicolor/scalable/apps/uk.co.grumlimited.authenticator-rs.svg
#	mkdir -p $(sharedir)/icons/hicolor/64x64/apps/
#	$(INSTALL_DATA) data/icons/hicolor/64x64/apps/uk.co.grumlimited.authenticator-rs.64.png $(sharedir)/icons/hicolor/64x64/apps/uk.co.grumlimited.authenticator-rs.png
#	mkdir -p $(sharedir)/icons/hicolor/128x128/apps/
#	$(INSTALL_DATA) data/icons/hicolor/128x128/apps/uk.co.grumlimited.authenticator-rs.128.png $(sharedir)/icons/hicolor/128x128/apps/uk.co.grumlimited.authenticator-rs.png
#
#	# Force icon cache refresh
#	touch $(sharedir)/icons/hicolor
#
#	# Install application metadata
#	mkdir -p $(sharedir)/metainfo/
#	$(INSTALL_DATA) data/uk.co.grumlimited.authenticator-rs.appdata.xml $(sharedir)/metainfo/uk.co.grumlimited.authenticator-rs.appdata.xml
#
#	# Install desktop file
#	mkdir -p $(sharedir)/applications/
#	$(INSTALL_DATA) data/uk.co.grumlimited.authenticator-rs.desktop $(sharedir)/applications/uk.co.grumlimited.authenticator-rs.desktop
#
#	# Install gschema file
#	mkdir -p $(sharedir)/glib-2.0/schemas/
#	$(INSTALL_DATA) data/uk.co.grumlimited.authenticator-rs.gschema.xml $(sharedir)/glib-2.0/schemas/

	# Install LOCALE files
	rm -fr builddir/
	meson setup builddir --prefix=$(PREFIX)
	meson install -C builddir --destdir=$(DESTDIR)

	echo XXX
	find $(DESTDIR)


# Install onto the system
install : release install-gresource
	# Install binary
	mkdir -p $(bindir)
	$(INSTALL_PROGRAM) target/release/authenticator-rs $(bindir)/authenticator-rs

# Remove an existing install from the system
uninstall :
	# Remove the desktop file
	rm -f $(sharedir)/applications/uk.co.grumlimited.authenticator-rs.desktop
	# Remove the application metadata
	rm -f $(sharedir)/metainfo/uk.co.grumlimited.authenticator-rs.appdata.xml
	# Remove gschema
	rm -f $(sharedir)/glib-2.0/schemas/uk.co.grumlimited.authenticator-rs.gschema.xml
	# Remove the icon
	rm -f $(sharedir)/icons/hicolor/scalable/apps/uk.co.grumlimited.authenticator-rs.svg
	rm -f $(sharedir)/icons/hicolor/64x64/apps/uk.co.grumlimited.authenticator-rs.png
	rm -f $(sharedir)/icons/hicolor/128x128/apps/uk.co.grumlimited.authenticator-rs.png
	# Remove the binary
	rm -f $(bindir)/bin/authenticator-rs

	# Remove LOCALE files
	find $(sharedir)/locale/ -name authenticator-rs.mo -exec rm {} \;

# Remove all files
clean-all : clean
	cargo clean

# Remove supplemental build files
clean :
	rm -rf target/*

debian-pkg : install
	mkdir -p $(DESTDIR)/DEBIAN
	cp build-aux/debian/control $(DESTDIR)/DEBIAN/
	echo "Version: $(RELEASE_VERSION)" >> $(DESTDIR)/DEBIAN/control
	cp build-aux/debian/postinst $(DESTDIR)/DEBIAN/
	chmod 775 $(DESTDIR)/DEBIAN/postinst
	dpkg-deb --build $(DESTDIR) authenticator-rs-$(RELEASE_VERSION)-x86_64.deb
	md5sum authenticator-rs-$(RELEASE_VERSION)-x86_64.deb > authenticator-rs-$(RELEASE_VERSION)-x86_64.deb.md5sum
