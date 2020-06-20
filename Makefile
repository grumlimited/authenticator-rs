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
target/release/authenticator-rs : src
	cargo build --release

# Install onto the system
install : target/release/authenticator-rs
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
	# Install icons
	$(INSTALL_DATA) data/uk.co.grumlimited.authenticator-rs.svg $(sharedir)/icons/hicolor/scalable/apps/uk.co.grumlimited.authenticator-rs.svg
	$(INSTALL_DATA) data/uk.co.grumlimited.authenticator-rs.64.png $(sharedir)/icons/hicolor/64x64/apps/uk.co.grumlimited.authenticator-rs.png
	$(INSTALL_DATA) data/uk.co.grumlimited.authenticator-rs.128.png $(sharedir)/icons/hicolor/128x128/apps/uk.co.grumlimited.authenticator-rs.png
	# Force icon cache refresh
	touch $(sharedir)/icons/hicolor
	# Install application meta-data
	$(INSTALL_DATA) data/uk.co.grumlimited.authenticator-rs.appdata.xml $(sharedir)/metainfo/uk.co.grumlimited.authenticator-rs.appdata.xml
	# Install desktop file
	$(INSTALL_DATA) data/uk.co.grumlimited.authenticator-rs.desktop $(sharedir)/applications/uk.co.grumlimited.authenticator-rs.desktop

# Remove an existing install from the system
uninstall :
	# Remove the desktop file
	rm -f $(sharedir)/applications/uk.co.grumlimited.authenticator-rs.desktop
	# Remove the application metadata
	rm -f $(sharedir)/metainfo/uk.co.grumlimited.authenticator-rs.appdata.xml
	# Remove the icon
	rm -f $(sharedir)/icons/hicolor/scalable/apps/uk.co.grumlimited.authenticator-rs.svg
	rm -f $(sharedir)/icons/hicolor/64x64/apps/uk.co.grumlimited.authenticator-rs.png
	rm -f $(sharedir)/icons/hicolor/128x128/apps/uk.co.grumlimited.authenticator-rs.png
	# Remove the binary
	rm -f $(bindir)/bin/authenticator-rs

# Remove all files
clean-all : clean
	cargo clean

# Remove supplemental build files
clean :
	rm -rf target/*

debian-pkg : install
	mkdir -p $(DESTDIR)/DEBIAN
	cp data/deb/control $(DESTDIR)/DEBIAN/
	echo "Version: $(RELEASE_VERSION)" >> $(DESTDIR)/DEBIAN/control
	cp data/deb/postinst $(DESTDIR)/DEBIAN/
	chmod 775 $(DESTDIR)/DEBIAN/postinst
	dpkg-deb --build $(DESTDIR) authenticator-rs-$(RELEASE_VERSION)-x86-64.deb

