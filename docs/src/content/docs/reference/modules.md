---
title: Module Reference
description: Per-module configuration options.
---

## clock

```toml
[[modules.right.right]]
type = "clock"
format = "%H:%M:%S"
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `format` | string | `"%H:%M"` | strftime format string |

## battery

```toml
[[modules.right.right]]
type = "battery"
```

No additional configuration needed. Displays level and charging state.

## cpu / memory / disk

```toml
[[modules.left.left]]
type = "cpu"
label = "CPU"
```

| Key | Type | Description |
|-----|------|-------------|
| `label` | string | Display label |

For `disk`, an additional `path` field specifies which mount point to monitor (default: `/`).

## weather

```toml
[[modules.right.right]]
type = "weather"
location = "auto"
interval = 600
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `location` | string | `"auto"` | Location name or `"auto"` |
| `interval` | int | `600` | Update interval in seconds |

## script

```toml
[[modules.right.right]]
type = "script"
command = "whoami"
interval = 60
```

| Key | Type | Description |
|-----|------|-------------|
| `command` | string | Shell command |
| `interval` | int | Seconds between runs |

## app_name / window_title

```toml
[[modules.left.left]]
type = "app_name"
max_length = 30
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `max_length` | int | `30` | Truncate after N characters |

## separator

```toml
[[modules.left.left]]
type = "separator"
```

Renders a vertical line between modules. No configuration needed.

## static_text

```toml
[[modules.left.left]]
type = "static_text"
text = "hello"
```

| Key | Type | Description |
|-----|------|-------------|
| `text` | string | Text to display |
