#!/bin/sh
# Publie une nouvelle version sur GitHub (ex. ./publier.sh v0.2.0) ; lui la récupère avec cosmic-discord-applet-maj.
set -e
cd "$(dirname "$0")"
[ -n "$1" ] || { echo "Usage : ./publier.sh v0.2.0"; exit 1; }
./paquet.sh
git push
gh release create "$1" dist/cosmic-discord-applet.tar.gz --title "$1" --generate-notes
