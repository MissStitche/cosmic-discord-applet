#!/bin/sh
# Publishes a new version on GitHub (e.g. ./publish.sh v0.2.0); he gets it with cosmic-discord-applet-update.
set -e
cd "$(dirname "$0")"
[ -n "$1" ] || { echo "Usage : ./publish.sh v0.2.0"; exit 1; }
./package.sh
git push
gh release create "$1" dist/cosmic-discord-applet.tar.gz --title "$1" --generate-notes
