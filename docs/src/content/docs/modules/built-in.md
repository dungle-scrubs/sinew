---
title: Built-in Modules
description: All modules that ship with Sinew.
---

## Display modules

| Module | Type | Description |
|--------|------|-------------|
| Clock | `clock` | Time display with configurable format |
| Date | `date` | Date display |
| DateTime | `datetime` | Combined date and time |
| App Name | `app_name` | Active application name |
| Window Title | `window_title` | Active window title |
| Static Text | `static_text` | Fixed text label |
| Separator | `separator` | Visual separator |

## System modules

| Module | Type | Description |
|--------|------|-------------|
| Battery | `battery` | Battery level and charging state |
| CPU | `cpu` | CPU usage percentage |
| Memory | `memory` | Memory usage |
| Disk | `disk` | Disk usage |
| Temperature | `temperature` | CPU/system temperature |
| Volume | `volume` | System volume with slider popup |
| WiFi | `wifi` | WiFi connection status |

## Rich modules

| Module | Type | Description |
|--------|------|-------------|
| Weather | `weather` | Weather with loading states |
| Now Playing | `now_playing` | Currently playing media |
| Calendar | `calendar` | Calendar popup |
| News | `news` | News feed |
| Script | `script` | Custom shell script output |
| API Usage | `api_usage` | API usage tracking |

## Example

```toml
[[modules.left.left]]
type = "app_name"
max_length = 30

[[modules.right.right]]
type = "battery"
label = "BAT"

[[modules.right.right]]
type = "clock"
format = "%H:%M:%S"
```
