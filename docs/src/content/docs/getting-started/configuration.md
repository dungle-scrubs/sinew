---
title: Configuration
description: Configure Sinew with a TOML config file.
---

Sinew loads its configuration from `~/.config/sinew/config.toml`. Changes are picked up automatically — no restart needed.

## Minimal config

```toml
[bar]
background_color = "#1D2123"
text_color = "#e2e2e2"
font_size = 13.0
font_family = "JetBrainsMono Nerd Font"
padding = 4.0

[[modules.left.left]]
type = "app_name"

[[modules.right.right]]
type = "clock"
format = "%H:%M"
```

## Bar settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `background_color` | string | `"#1e1e2e"` | Bar background color |
| `text_color` | string | `"#cdd6f4"` | Default text color |
| `font_family` | string | `"SF Pro"` | Font family |
| `font_size` | float | `13.0` | Font size in points |
| `height` | string | `"auto"` | Bar height (`"auto"` or pixel value) |
| `padding` | float | `4.0` | Vertical padding |

## Module positions

Modules are placed in four zones:

```toml
# Far left
[[modules.left.left]]
type = "app_name"

# Left of notch
[[modules.left.right]]
type = "static_text"
text = "inner-left"

# Right of notch
[[modules.right.left]]
type = "static_text"
text = "inner-right"

# Far right
[[modules.right.right]]
type = "clock"
```

See [Layout & Zones](/guides/layout/) for a visual explanation of the four-zone system.
