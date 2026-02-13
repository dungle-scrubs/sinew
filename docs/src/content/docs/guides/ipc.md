---
title: IPC Commands
description: Control Sinew at runtime via Unix socket.
---

Sinew exposes a Unix socket at `/tmp/sinew.sock` for runtime control.

## Using sinew-msg

```bash
# Force a redraw
sinew-msg redraw

# Reload configuration
sinew-msg reload

# Get status as JSON
sinew-msg status
```

## Available commands

| Command | Description |
|---------|-------------|
| `redraw` | Force an immediate bar redraw |
| `reload` | Reload config from disk |
| `status` | Return JSON with current state |

## From source

If you built from source and haven't installed `sinew-msg` globally:

```bash
cargo run --bin sinew-msg -- redraw
cargo run --bin sinew-msg -- reload
cargo run --bin sinew-msg -- status
```
