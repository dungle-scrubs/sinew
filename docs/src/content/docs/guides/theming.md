---
title: Theming
description: Customize the look and feel of your Sinew bar.
---

Sinew supports full color theming through the `[bar]` config section.

## Colors

All colors are specified as hex strings:

```toml
[bar]
background_color = "#1D2123"
text_color = "#e2e2e2"
border_color = "#5f656a"
```

## Fonts

```toml
[bar]
font_family = "JetBrainsMono Nerd Font"
font_size = 13.0
```

Any font installed on your system can be used. Nerd Fonts are recommended for icon support in modules.

## Per-module styling

Individual modules can override global styles:

```toml
[[modules.right.right]]
type = "battery"
text_color = "#a6e3a1"
padding_left = 8.0
padding_right = 8.0
```
