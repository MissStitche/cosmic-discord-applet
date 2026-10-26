#!/bin/sh
# Met à jour l'applet avec la dernière version publiée sur GitHub.
set -e
dossier=$(mktemp -d)
trap 'rm -rf "$dossier"' EXIT
curl -fsSL https://github.com/MissStitche/cosmic-discord-applet/releases/latest/download/cosmic-discord-applet.tar.gz | tar -xz -C "$dossier"
"$dossier/cosmic-discord-applet/install.sh"
