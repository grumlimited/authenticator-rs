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
target/release/gDiceRoller : src
	cargo build --release

# Install onto the system
install : target/release/gDiceRoller
	# Create the bindir, if need be
	mkdir -p $(bindir)
	# Install binary
	$(INSTALL_PROGRAM) target/release/gDiceRoller $(bindir)/codes.nora.gDiceRoller
	# Create the sharedir and subfolders, if need be
	mkdir -p $(sharedir)/icons/hicolor/scalable/apps/
	mkdir -p $(sharedir)/icons/hicolor/64x64/apps/
	mkdir -p $(sharedir)/icons/hicolor/128x128/apps/
	mkdir -p $(sharedir)/applications/
	mkdir -p $(sharedir)/metainfo/
	# Install icons
	$(INSTALL_DATA) data/codes.nora.gDiceRoller.svg $(sharedir)/icons/hicolor/scalable/apps/codes.nora.gDiceRoller.svg
	$(INSTALL_DATA) data/codes.nora.gDiceRoller.64.png $(sharedir)/icons/hicolor/64x64/apps/codes.nora.gDiceRoller.png
	$(INSTALL_DATA) data/codes.nora.gDiceRoller.128.png $(sharedir)/icons/hicolor/128x128/apps/codes.nora.gDiceRoller.png
	# Force icon cache refresh
	touch $(sharedir)/icons/hicolor
	# Install application meta-data
	$(INSTALL_DATA) data/codes.nora.gDiceRoller.appdata.xml $(sharedir)/metainfo/codes.nora.gDiceRoller.appdata.xml
	# Install desktop file
	$(INSTALL_DATA) data/codes.nora.gDiceRoller.desktop $(sharedir)/applications/codes.nora.gDiceRoller.desktop

# Remove an existing install from the system
uninstall :
	# Remove the desktop file
	rm -f $(sharedir)/applications/codes.nora.gDiceRoller.desktop
	# Remove the application metadata
	rm -f $(sharedir)/metainfo/codes.nora.gDiceRoller.appdata.xml
	# Remove the icon
	rm -f $(sharedir)/icons/hicolor/scalable/apps/codes.nora.gDiceRoller.svg
	rm -f $(sharedir)/icons/hicolor/64x64/apps/codes.nora.gDiceRoller.png
	rm -f $(sharedir)/icons/hicolor/128x128/apps/codes.nora.gDiceRoller.png
	# Remove the binary
	rm -f $(bindir)/bin/codes.nora.gDiceRoller

# Remove all files
clean-all : clean
	cargo clean

# Remove supplemental build files
clean :
	rm -rf target/*

