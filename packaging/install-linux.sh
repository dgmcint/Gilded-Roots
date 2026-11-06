#!/bin/sh
# Gilded Roots for Linux: install for this user (no administrator password needed).
#
#   ./install.sh              install, or update an existing install
#   ./install.sh --uninstall  remove the program (your family tree is never removed)
#
# The program goes in ~/.local/bin, with a menu entry and icon. Your family tree is kept in
# ~/Documents/Gilded Roots. For a copy that runs from a USB drive instead, use the
# "Portable (USB drive)" folder next to this script.

set -e
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BIN="$HOME/.local/bin"
APPS="$HOME/.local/share/applications"
ICONS="$HOME/.local/share/icons/hicolor/256x256/apps"

if [ "$1" = "--uninstall" ]; then
    rm -f "$BIN/gilded-roots" "$APPS/gilded-roots.desktop" "$ICONS/gilded-roots.png"
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS" >/dev/null 2>&1 || true
    echo "Gilded Roots has been removed. Your family tree in ~/Documents/Gilded Roots was left in place."
    exit 0
fi

mkdir -p "$BIN" "$APPS" "$ICONS"
cp "$HERE/gilded-roots" "$BIN/gilded-roots"
chmod 755 "$BIN/gilded-roots"
cp "$HERE/gilded-roots.png" "$ICONS/gilded-roots.png"
cat > "$APPS/gilded-roots.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Gilded Roots
Comment=Family tree and genealogy
Exec="$BIN/gilded-roots"
Icon=gilded-roots
Terminal=false
Categories=Office;Education;
EOF
chmod 644 "$APPS/gilded-roots.desktop"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS" >/dev/null 2>&1 || true

echo "Gilded Roots is installed. Find it with your other applications, or run: $BIN/gilded-roots"
echo "To remove it later:  ./install.sh --uninstall"
