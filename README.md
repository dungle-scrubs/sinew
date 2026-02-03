# RustyBar

[![CI](https://github.com/dungle-scrubs/rustybar/actions/workflows/ci.yml/badge.svg)](https://github.com/dungle-scrubs/rustybar/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A macOS menu bar replacement built in Rust. Features notch-aware split layouts, modular widgets, and hot-reload configuration.

## Features

- **Notch-aware layout** - Automatically splits around MacBook notch, or use fake notch on external displays
- **Four-zone module placement** - Left outer, left inner, right inner, right outer
- **Hot-reload configuration** - Edit config and see changes instantly
- **Built-in modules** - Clock, date, battery, volume, CPU, memory, weather, app name, and more
- **Hisohiso waveform** - Optional dictation waveform module via local IPC
- **Popups and panels** - Calendar popups, script-driven content panels
- **Toggle groups** - Radio-button style module groups with active state styling
- **Autohide support** - Slides down when macOS menu bar appears

## Requirements

- macOS 12.0 (Monterey) or later
- Rust 1.75+

## Installation

### Homebrew (recommended)

```bash
brew install dungle-scrubs/rustybar/rustybar
```

### Building from Source

```bash
git clone https://github.com/dungle-scrubs/rustybar.git
cd rustybar
cargo build --release
cp target/release/rustybar /usr/local/bin/
```

## Usage

```bash
# Run with default config (~/.config/rustybar/config.toml)
rustybar

# Run with debug logging
RUST_LOG=debug rustybar
```

### IPC Commands

```bash
# Force redraw
cargo run --bin rustybar-msg -- redraw

# Reload configuration
cargo run --bin rustybar-msg -- reload

# Get status
cargo run --bin rustybar-msg -- status
```

## Configuration

Create `~/.config/rustybar/config.toml`:

```toml
[bar]
background_color = "#1D2123"
text_color = "#e2e2e2"
font_size = 13.0
font_family = "JetBrainsMono Nerd Font"
padding = 4.0
border_color = "#5f656a"
border_width = 1.0
border_radius = 6.0
autohide = true  # Slide down when macOS menu bar appears

# Left side modules
[[modules.left.left]]
type = "app_name"
max_length = 30
background = "#2C3135"
padding = 8.0
corner_radius = 6.0

# Right side modules (left to right order)
[[modules.right.right]]
type = "weather"
location = "auto"
update_interval = 600
color = "#50cae5"

[[modules.right.right]]
type = "battery"
color = "#8ec475"
warning_color = "#dcc37c"
warning_threshold = 30
critical_color = "#e8626f"
critical_threshold = 15

[[modules.right.right]]
type = "clock"
format = "%H:%M"
background = "#2C3135"
padding = 8.0
corner_radius = 6.0
```

See [config.example.toml](config.example.toml) for all options.

### Module Types

| Type | Description |
|------|-------------|
| `clock` | Time display with custom format |
| `date` | Date display with optional calendar popup |
| `battery` | Battery percentage with threshold colors |
| `volume` | System volume level |
| `cpu` | CPU usage percentage |
| `hisohiso` | Hisohiso dictation waveform via local IPC |
| `memory` | Memory usage |
| `weather` | Weather from wttr.in |
| `app_name` | Active application name |
| `static` | Static text with optional icon |
| `script` | Custom script output |
| `separator` | Space, line, dot, or icon separator |

## Known Limitations

- **macOS only** - Uses AppKit/Core Graphics directly
- **Weather uses wttr.in** - No API key required, but rate limits apply
- **No multi-monitor support yet** - Shows on primary display only

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

[MIT](LICENSE)
