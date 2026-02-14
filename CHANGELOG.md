# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0](https://github.com/dungle-scrubs/sinew/compare/v0.2.0...v0.3.0) (2026-02-14)


### Features

* add blueprint logo and macOS app icon ([875107e](https://github.com/dungle-scrubs/sinew/commit/875107ed55081bef8f9cd3c5c316c9162bb7ca6c))
* hide from Dock and set app icon at launch ([3288934](https://github.com/dungle-scrubs/sinew/commit/32889344d15acd63d0ac43f4a8399661dcdd54a0))
* IPC-driven module extensibility with external module type ([38b5cc6](https://github.com/dungle-scrubs/sinew/commit/38b5cc64ecb2849055b02b59a22ca2af4f654d1e))


### Bug Fixes

* dark mode contrast for sidebar links and body text ([5a78179](https://github.com/dungle-scrubs/sinew/commit/5a78179de6514169f05ed17a3143958fdb23e364))
* resolve all clippy warnings for clean CI ([8c04e95](https://github.com/dungle-scrubs/sinew/commit/8c04e9507f08c87d6277713dd875404fe362ed87))

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
