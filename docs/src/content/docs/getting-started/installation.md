---
title: Installation
description: Install Sinew on macOS via Homebrew or from source.
---

## Homebrew (recommended)

```bash
brew install dungle-scrubs/sinew/sinew
```

### Run at login

```bash
brew services start sinew
```

This registers a launchd service that starts Sinew automatically on login.

## Building from source

### Requirements

- macOS 12.0 (Monterey) or later
- Rust 1.75+

### Steps

```bash
git clone https://github.com/dungle-scrubs/sinew.git
cd sinew
cargo build --release
cp target/release/sinew /usr/local/bin/
```
