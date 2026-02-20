# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.3](https://github.com/dungle-scrubs/sinew/compare/v0.3.2...v0.3.3) (2026-02-20)


### Bug Fixes

* **release:** sync Cargo.toml to 0.3.2 ([7f8731f](https://github.com/dungle-scrubs/sinew/commit/7f8731ff41048f699b5505d8fe95ff978a9f47a9))
* **release:** sync Cargo.toml to 0.3.2 ([66bcd6f](https://github.com/dungle-scrubs/sinew/commit/66bcd6f648963d67d74bed2ef7d873e37f866c9b))

## [0.3.2](https://github.com/dungle-scrubs/sinew/compare/v0.3.1...v0.3.2) (2026-02-20)


### Bug Fixes

* **gpui:** align auto bar geometry and prevent startup popup artifacts ([6df4fdf](https://github.com/dungle-scrubs/sinew/commit/6df4fdfca38ae3a933a5f822f6920af88ec8ce74))
* **gpui:** pin auto bar to menu edge and prevent startup popup artifacts ([0317f78](https://github.com/dungle-scrubs/sinew/commit/0317f78d91f634bf19e8a32e54d56784fa9b946a))

## [0.3.1](https://github.com/dungle-scrubs/sinew/compare/v0.3.0...v0.3.1) (2026-02-19)


### Bug Fixes

* **bar:** honor camera config and harden clicks ([3afd6d5](https://github.com/dungle-scrubs/sinew/commit/3afd6d542c28f8614d7a396db4d655439ea42b95))
* **ipc:** handle quoted args and strict triggers ([051a391](https://github.com/dungle-scrubs/sinew/commit/051a391e999398846ea0560e99013f2a356ab78e))
* **popup:** defer AppKit mutations and add Esc close ([70e1b72](https://github.com/dungle-scrubs/sinew/commit/70e1b725ec22562b12283cf52238491a85b39218))
* stabilize popup windowing and harden IPC ([1b74ef5](https://github.com/dungle-scrubs/sinew/commit/1b74ef56bd6fbcbf56c886d3b7cf62328c5cf009))

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
