# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-02-15

### Added

- IPC-driven module extensibility with external module type
- Hide from Dock and set app icon at launch
- Blueprint logo and macOS app icon

### Fixed

- App name module not updating on app switch (MainThreadMarker unavailable off main thread)
- Weather and other background-thread modules not updating (refresh timer not polling dirty flags)
- Dark mode contrast for sidebar links and body text
- Resolve all clippy warnings for clean CI

### Documentation

- Add Starlight docs site with blueprint theme
- Add screenshot and logo to README
- Add GitHub share images

### Maintenance

- Add Husky pre-commit with Biome linting

## [0.1.0] - 2026-01-30

### Added

- Initial release
- Notch-aware split bar layout for MacBook displays
- Four-zone module placement (left outer/inner, right outer/inner)
- Hot-reload configuration via `~/.config/sinew/config.toml`
- Built-in modules: clock, date, battery, volume, CPU, memory, weather, app name, static text, script, separator
- Calendar popup for date module
- Toggle groups with radio-button behavior
- Autohide support for macOS menu bar coexistence
- IPC server for external control (redraw, reload, status)
- Configurable colors, fonts, borders, and corner radius
- Threshold-based coloring for battery module
