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

# Build the application
release : src
	cargo build --release

# Compiling gResource
gresource:
	glib-compile-resources data/uk.co.grumlimited.authenticator-rs.xml

run: gresource
	cargo run

clippy:
	find src/ -name "*.rs" -exec touch {} \;
	cargo clippy

release-version:
	sed -i 's/#VERSION_NUMBER#/$(RELEASE_VERSION)/' ./data/resources/gtk/ui/main.ui

install-po: # dev only - run with sudo
	msgfmt po/fr.po -o /usr/share/locale/fr/LC_MESSAGES/authenticator-rs.mo
	msgfmt po/en_GB.po -o /usr/share/locale/en_GB/LC_MESSAGES/authenticator-rs.mo

# Install onto the system
install : release gresource
	# Create the bindir, if need be
	mkdir -p $(bindir)
	# Install binary
	$(INSTALL_PROGRAM) target/release/authenticator-rs $(bindir)/authenticator-rs

	# Create the sharedir and subfolders, if need be
	mkdir -p $(sharedir)/icons/hicolor/scalable/apps/
	mkdir -p $(sharedir)/icons/hicolor/64x64/apps/
	mkdir -p $(sharedir)/icons/hicolor/128x128/apps/
	mkdir -p $(sharedir)/applications/
	mkdir -p $(sharedir)/metainfo/
	mkdir -p $(sharedir)/uk.co.grumlimited.authenticator-rs/
	mkdir -p $(sharedir)/glib-2.0/schemas/

	# Install gResource
	$(INSTALL_DATA) data/uk.co.grumlimited.authenticator-rs.gresource $(sharedir)/uk.co.grumlimited.authenticator-rs/uk.co.grumlimited.authenticator-rs.gresource
	
	# Install icons
	$(INSTALL_DATA) data/icons/hicolor/scalable/apps/uk.co.grumlimited.authenticator-rs.svg $(sharedir)/icons/hicolor/scalable/apps/uk.co.grumlimited.authenticator-rs.svg
	$(INSTALL_DATA) data/icons/hicolor/64x64/apps/uk.co.grumlimited.authenticator-rs.64.png $(sharedir)/icons/hicolor/64x64/apps/uk.co.grumlimited.authenticator-rs.png
	$(INSTALL_DATA) data/icons/hicolor/128x128/apps/uk.co.grumlimited.authenticator-rs.128.png $(sharedir)/icons/hicolor/128x128/apps/uk.co.grumlimited.authenticator-rs.png

	# Force icon cache refresh
	touch $(sharedir)/icons/hicolor

	# Install application meta-data
	$(INSTALL_DATA) data/uk.co.grumlimited.authenticator-rs.appdata.xml $(sharedir)/metainfo/uk.co.grumlimited.authenticator-rs.appdata.xml

	# Install desktop file
	$(INSTALL_DATA) data/uk.co.grumlimited.authenticator-rs.desktop $(sharedir)/applications/uk.co.grumlimited.authenticator-rs.desktop

	# Install gschema file
	$(INSTALL_DATA) data/uk.co.grumlimited.authenticator-rs.gschema.xml $(sharedir)/glib-2.0/schemas/
	
	# Install LOCALE files
	rm -fr builddir/
	meson builddir --prefix=/usr
	DESTDIR=../build-aux/i18n meson install -C builddir
	cp -fr build-aux/i18n/usr $(DESTDIR)

# Remove an existing install from the system
uninstall :
	# Remove the desktop file
	rm -f $(sharedir)/applications/uk.co.grumlimited.authenticator-rs.desktop
	# Remove the application metadata
	rm -f $(sharedir)/metainfo/uk.co.grumlimited.authenticator-rs.appdata.xml
	# Remove gschema
	rm -f /usr/share/glib-2.0/schemas/uk.co.grumlimited.authenticator-rs.gschema.xml
	# Remove the icon
	rm -f $(sharedir)/icons/hicolor/scalable/apps/uk.co.grumlimited.authenticator-rs.svg
	rm -f $(sharedir)/icons/hicolor/64x64/apps/uk.co.grumlimited.authenticator-rs.png
	rm -f $(sharedir)/icons/hicolor/128x128/apps/uk.co.grumlimited.authenticator-rs.png
	# Remove the binary
	rm -f $(bindir)/bin/authenticator-rs

	# Remove LOCALE files
	find /usr/share/locale/ -name authenticator-rs.mo -exec rm {} \;

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

