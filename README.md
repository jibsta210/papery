# Papery

A native wallpaper changer for [COSMIC Desktop](https://system76.com/cosmic), built with Rust and libcosmic.

Papery automatically rotates your desktop background using images from multiple online sources, with a system tray icon for quick control.

## Features

- **Multiple sources** — Bing Photo of the Day, NASA APOD, Wallhaven, Google Earth View, local folders
- **System tray** — Runs in the background with tray controls (next, pause, quit)
- **Smart scheduling** — Configurable interval with H:M:S precision
- **Theme filtering** — Filter wallpapers by brightness (light/dark/any)
- **Scaling modes** — Zoom, Fit, or Stretch
- **History & favorites** — Browse past wallpapers and save favorites
- **Auto-start** — Launches on login, rotates wallpapers in the background

## Screenshots

*Coming soon*

## Install

### From source

```bash
# Dependencies (Debian/Ubuntu/Pop!_OS)
sudo apt install cargo cmake just libexpat1-dev libfontconfig-dev libfreetype-dev libxkbcommon-dev pkg-config

# Build and install
git clone https://github.com/jakes/papery.git
cd papery
just install
```

### Arch / CachyOS

```bash
git clone https://github.com/jakes/papery.git
cd papery
cargo build --release
sudo install -Dm755 target/release/papery /usr/local/bin/papery
sudo install -Dm644 data/dev.papery.CosmicApplet.desktop /usr/share/applications/dev.papery.CosmicApplet.desktop
sudo install -Dm644 data/icons/scalable/apps/dev.papery.CosmicApplet.svg /usr/share/icons/hicolor/scalable/apps/dev.papery.CosmicApplet.svg
install -Dm644 data/dev.papery.CosmicApplet-autostart.desktop ~/.config/autostart/dev.papery.CosmicApplet.desktop
```

## Usage

```bash
# Open the settings window
papery

# Run in background only (tray + wallpaper rotation)
papery --bg
```

- **Close the window** — Papery continues running in the background via the system tray
- **Tray menu** — Right-click the tray icon for Next Wallpaper, Pause/Resume, Show Papery, Quit
- **Auto-start** — On login, `papery --bg` runs automatically

## How It Works

Papery writes to the COSMIC background config (`~/.config/cosmic/com.system76.CosmicBackground/`), which `cosmic-bg` picks up via inotify. Downloaded wallpapers are cached in `~/.cache/papery/`.

## Wallpaper Sources

| Source | API Key Required | Description |
|--------|:---:|---|
| Bing Photo of the Day | No | Daily curated photos |
| NASA APOD | No (uses DEMO_KEY) | Astronomy images |
| Wallhaven | No | Community wallpapers |
| Google Earth View | No | Satellite imagery |
| Local Folders | N/A | Your own images |

## Configuration

Settings are stored via `cosmic-config` at `~/.config/cosmic/dev.papery.CosmicApplet/v1/`.

## License

GPL-3.0-only
