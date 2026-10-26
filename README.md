# Applet COSMIC Discord

Icône dans le panneau COSMIC qui montre le vocal Discord (salon, qui parle, muet).
Utilise le pont Rust de [quickshell-config](https://github.com/MissStitche/quickshell-config) (`discord-bridge-rs/`).

## Installer (première fois)

```sh
curl -fsSL https://github.com/MissStitche/cosmic-discord-applet/releases/latest/download/cosmic-discord-applet.tar.gz | tar -xz -C /tmp && /tmp/cosmic-discord-applet/install.sh
```

Puis Paramètres > Bureau > Panneau > Applets > ajouter « Discord (widget) ».

## Mettre à jour

```sh
~/.local/bin/cosmic-discord-applet-update
```
