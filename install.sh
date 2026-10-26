#!/bin/sh
# Installs the applet for the current user (no sudo needed).
set -e
cd "$(dirname "$0")"
mkdir -p ~/.local/bin ~/.local/share/applications
install -m 755 cosmic-discord-applet discord-bridge ~/.local/bin/
install -m 755 update.sh ~/.local/bin/cosmic-discord-applet-update
# Old French name of the update command (before v0.4.0)
rm -f ~/.local/bin/cosmic-discord-applet-maj
cat > ~/.local/share/applications/fr.chloe.CosmicDiscordApplet.desktop <<DESKTOP
[Desktop Entry]
Name=Discord (widget)
Comment=Vocal Discord dans le panneau
Type=Application
Exec=$HOME/.local/bin/cosmic-discord-applet
Icon=audio-headset-symbolic
Terminal=false
Categories=COSMIC;
NoDisplay=true
X-CosmicApplet=true
X-CosmicHoverPopup=Auto
DESKTOP
# Stop the old version: the panel restarts the new one.
pkill -f "$HOME/.local/bin/cosmic-discord-applet" 2>/dev/null || true
pkill -f "$HOME/.local/bin/discord-bridge daemon" 2>/dev/null || true
echo "Installé ! Ajoute « Discord (widget) » dans Paramètres > Bureau > Panneau > Applets."
echo "Mises à jour : ~/.local/bin/cosmic-discord-applet-update"
