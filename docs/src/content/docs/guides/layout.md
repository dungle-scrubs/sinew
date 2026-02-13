---
title: Layout & Zones
description: Understanding Sinew's four-zone notch-aware layout.
---

Sinew splits the menu bar into two sections around the MacBook notch (or a configurable gap on external displays).

## The four zones

```
┌─────────────────────┐   ┌─────────────────────┐
│ left.left ◄─space─► │   │ ◄─space─► right.right│
│           left.right │   │ right.left            │
└─────────────────────┘   └─────────────────────┘
        LEFT SECTION    NOTCH    RIGHT SECTION
```

| Zone | Config key | Position | Typical use |
|------|-----------|----------|-------------|
| Left outer | `modules.left.left` | Far left | App name, system stats |
| Left inner | `modules.left.right` | Left of notch | Modules near center |
| Right inner | `modules.right.left` | Right of notch | Modules near center |
| Right outer | `modules.right.right` | Far right | Clock, battery, weather |

Each section uses flexbox with a spacer between its outer and inner zone, pushing modules toward their respective edges.

## Notch gap

The notch gap is a fixed 200px width between the left and right sections. On displays without a notch, this creates a clean center divide.

## External displays

On external monitors (no physical notch), Sinew can optionally render a "fake notch" gap to maintain the same layout, or run as a single full-width bar.
