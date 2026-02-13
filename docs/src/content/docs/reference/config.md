---
title: Config Reference
description: Complete configuration reference for Sinew.
---

Configuration lives at `~/.config/sinew/config.toml`.

## `[bar]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `height` | string | `"auto"` | Bar height — `"auto"` or pixel value |
| `background_color` | string | `"#1e1e2e"` | Background color |
| `text_color` | string | `"#cdd6f4"` | Default text color |
| `font_family` | string | `"SF Pro"` | Font family |
| `font_size` | float | `13.0` | Font size in points |
| `padding` | float | `4.0` | Vertical padding |
| `border_color` | string | — | Border color |
| `border_radius` | float | `0.0` | Corner radius |
| `hover_effects` | bool | `true` | Enable hover effects |
| `camera_indicator` | bool | `false` | Show camera recording indicator |

## `[[modules.<position>]]`

Positions: `left.left`, `left.right`, `right.left`, `right.right`

### Common fields

| Key | Type | Description |
|-----|------|-------------|
| `type` | string | Module type (required) |
| `label` | string | Text label |
| `label_align` | string | `"left"` or `"right"` |
| `fixed_width` | float | Fixed width in pixels |
| `padding_left` | float | Left padding |
| `padding_right` | float | Right padding |
| `text_color` | string | Override text color |
| `show_while_loading` | bool | Show during initial load |

### Module-specific fields

See [Module Reference](/reference/modules/) for per-module options.
