#!/bin/sh
# Updates the applet to the latest version published on GitHub.
set -e
folder=$(mktemp -d)
trap 'rm -rf "$folder"' EXIT
curl -fsSL https://github.com/MissStitche/cosmic-discord-applet/releases/latest/download/cosmic-discord-applet.tar.gz | tar -xz -C "$folder"
"$folder/cosmic-discord-applet/install.sh"
