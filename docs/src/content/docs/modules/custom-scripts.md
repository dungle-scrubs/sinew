---
title: Custom Scripts
description: Run shell scripts as Sinew modules.
---

The `script` module runs a shell command on an interval and displays its output in the bar.

## Configuration

```toml
[[modules.right.right]]
type = "script"
command = "echo $(date +%s)"
interval = 5
label = "epoch"
```

| Key | Type | Description |
|-----|------|-------------|
| `command` | string | Shell command to execute |
| `interval` | int | Seconds between executions |
| `label` | string | Optional label prefix |

## Guidelines

- Commands run **off the main thread** — they won't block the UI
- Keep commands fast (under 100ms) for best responsiveness
- Output is trimmed to a single line for bar display
- Use `script` for anything Sinew doesn't have a built-in module for
