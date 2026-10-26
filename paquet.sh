#!/bin/sh
# Compile l'applet et prépare l'archive à envoyer (copiée dans Téléchargements pour Discord Flatpak).
set -e
cd "$(dirname "$0")"
cargo build --release
rm -rf dist && mkdir -p dist/cosmic-discord-applet
cp target/release/cosmic-discord-applet ../quickshell-config/discord-bridge-rs/target/release/discord-bridge install.sh maj.sh dist/cosmic-discord-applet/
strip dist/cosmic-discord-applet/cosmic-discord-applet dist/cosmic-discord-applet/discord-bridge
tar -czf dist/cosmic-discord-applet.tar.gz -C dist cosmic-discord-applet
cp dist/cosmic-discord-applet.tar.gz ~/Téléchargements/
echo "Archive prête : ~/Téléchargements/cosmic-discord-applet.tar.gz"
