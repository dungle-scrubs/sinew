---
title: Modules Overview
description: How Sinew's module system works.
---

Sinew's bar is composed of **modules** — self-contained widgets that render in one of four zones. Each module has a type, optional configuration, and can provide popups.

## Module lifecycle

1. Config is loaded and modules are instantiated via the factory registry
2. Each module's `update()` is called on a schedule (or via events)
3. `render()` produces the bar element; `render_popup()` produces popup content
4. On config change, modules are replaced — `on_module_stop()` then `on_module_start()`

## Common module options

Every module supports these fields:

| Key | Type | Description |
|-----|------|-------------|
| `type` | string | Module type (required) |
| `label` | string | Optional text label |
| `label_align` | string | `"left"` or `"right"` |
| `fixed_width` | float | Fixed width in pixels |
| `padding_left` | float | Left padding |
| `padding_right` | float | Right padding |
