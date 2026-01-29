# RustyBar

A SketchyBar alternative built in pure Rust using AppKit (objc2/cocoa crates).

## Goals

- **Replace the macOS menu bar** — visible top bar, system bar auto-hidden
- **Notch-aware** — split around MacBook notch, adapt per-monitor
- **Four-zone layout** — left-left, left-right, right-left, right-right
- **Modular** — reorderable modules in any zone
- Lightweight (sub-10MB memory)
- Hot-reload configuration
- Integration with Aerospace and tiling WMs

## Layout System

### Two-Half Design (Always Split)

```
With notch (MacBook built-in display):
┌─────────────────────┐         ┌─────────────────────┐
│ LEFT-L    LEFT-R    │ [NOTCH] │ RIGHT-L    RIGHT-R  │
└─────────────────────┘         └─────────────────────┘

Without notch (external monitors) — still split at center:
┌─────────────────────┐         ┌─────────────────────┐
│ LEFT-L    LEFT-R    │ [GAP/FAKE NOTCH] │ RIGHT-L    RIGHT-R  │
└─────────────────────┘         └─────────────────────┘

With fake notch enabled (aesthetic preference):
┌─────────────────────╮         ╭─────────────────────┐
│ LEFT-L    LEFT-R    │  NOTCH  │ RIGHT-L    RIGHT-R  │
└─────────────────────┴─────────┴─────────────────────┘
```

**Always two halves**, even on external monitors. Optional fake notch for visual consistency.

### Fake Notch

Draws a notch shape matching the real MacBook notch aesthetic — hangs down from top:

```
══════════════════════════════════════════════  ← screen top edge
                      ┌───────┐
    ──────────────────╯       ╰──────────────   ← top corners curve OUTWARD
    │   LEFT BAR                 RIGHT BAR  │
    │                                       │
    ─────────────────────────────────────────

Close-up of notch shape:

    ────────╮             ╭────────   ← curves outward into bar area
            │             │
            │   NOTCH     │           ← configurable width & color
            │             │
            ╰─────────────╯           ← bottom corners curve inward (rounded)
```

- Creates an **exclusion zone** — modules respect this boundary
- Configurable width, color, and corner radius
- Matches the real notch proportions by default

```toml
[bar.notch]
fake = true
width = 200
color = "#000000"          # Match screen bezel or customize
corner_radius = 8          # Curves at bottom corners
```

### Four Zones

| Zone | Alignment | Typical Use |
|------|-----------|-------------|
| `left.left` | Left edge, grow right | Workspaces, app icon |
| `left.right` | Center-left, grow left | Window title |
| `right.left` | Center-right, grow right | System stats |
| `right.right` | Right edge, grow left | Clock, battery |

### Flex Layout Model

Modules behave like CSS flexbox — fixed or flexible widths:

```
┌──────────────────────────────────────────────────────┐
│ [fixed]  [fixed]  [―――― flex ――――]  [fixed] [fixed] │
└──────────────────────────────────────────────────────┘
```

```rust
enum ModuleWidth {
    Fixed,                    // Natural width, doesn't shrink/grow
    Flex { min: f64, max: f64 }, // Grows/shrinks within bounds
}
```

**Fixed modules**: Icons, badges, stats — take their natural width
**Flex modules**: Window title, spacers — grow to fill, shrink when needed

```toml
[[modules.left.right]]
type = "window_title"
flex = true
min_width = 50
max_width = 400        # Won't grow beyond this
truncate = "middle"    # "start", "middle", "end"
```

When space is constrained, flex modules shrink first. If still not enough, they hit `min_width` and truncate their content.

**Dynamic content width**: When module content changes (e.g., "Mon" → "Wednesday", or "5%" → "100%"), the layout smoothly transitions using the same `transition_ms` timing. Fixed-width modules grow/shrink to fit their new content, and neighboring modules adjust accordingly.

```
Before:  [󰁹 5%]  [Mon]  [10:05 AM]
After:   [󰁹 100%]  [Wednesday]  [10:05 AM]
         ↑ smooth width transition
```

### Menu Bar Height Detection

```rust
fn system_menubar_height() -> f64 {
    // NSStatusBar.system.thickness gives exact menu bar height
    NSStatusBar::system().thickness()
}

// Or calculate from screen geometry
fn menubar_height_from_screen(screen: &NSScreen) -> f64 {
    let frame = screen.frame();
    let visible = screen.visibleFrame();
    // Menu bar height = total height - visible height - dock (if at bottom)
    frame.height - visible.height - visible.origin.y
}
```

Menu bar height varies:
- **Non-notch Macs**: ~24px
- **Notch Macs**: ~37px (taller to accommodate notch)
- Can vary with display scaling

Using `"auto"` ensures RustyBar matches exactly.

### Menu Bar Auto-Hide Detection

When the system menu bar is set to auto-hide, RustyBar can sync its visibility — fading in/out alongside it.

**Detection approaches:**

1. **NSScreen.visibleFrame observation** (most reliable public API)
   ```rust
   // visibleFrame changes when menu bar shows/hides
   // Set up KVO or poll frequently during suspected transitions
   fn observe_menubar_visibility() {
       NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
           observer,
           sel!(screenParametersChanged:),
           NSApplicationDidChangeScreenParametersNotification,
           None,
       );
   }

   fn is_menubar_visible(screen: &NSScreen) -> bool {
       let frame = screen.frame();
       let visible = screen.visibleFrame();
       // If visible.height + visible.origin.y < frame.height, menu bar is showing
       (frame.height - visible.height - visible.origin.y) > 1.0
   }
   ```

2. **Polling during transition** — Check visibleFrame every ~16ms when transition suspected

3. **Private APIs** — `CGSConnection` may have menu bar state notifications (undocumented)

**Sync behavior:**
```toml
[bar]
sync_with_menubar = true
sync_animation = "slide"   # "slide" (slot machine) or "fade"
sync_duration = 200        # Match system animation (~200ms)
```

**Animation styles:**

1. **Slide (slot machine)** — RustyBar and menu bar share vertical space, push each other out
   ```
   Menu bar slides down → RustyBar pushed down, clips at bottom edge
   Menu bar slides up   → RustyBar slides up from below

   ┌─────────────────────────┐
   │    SYSTEM MENU BAR      │ ← slides down
   ├ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┤
   │    RUSTYBAR (clipped)   │ ← pushed out, clips at bottom
   └─────────────────────────┘
   ```

2. **Fade** — RustyBar fades out when menu bar appears, fades in when it hides

```rust
fn on_menubar_transition(&self, menubar_y: f64) {
    match self.config.sync_animation {
        SyncAnimation::Slide => {
            // Move RustyBar down by menubar's current height
            // Clip at bottom edge (NSView.clipsToBounds or layer mask)
            let offset = menubar_y;
            self.window.setFrameOrigin(NSPoint::new(0.0, screen_height - bar_height - offset));
        }
        SyncAnimation::Fade => {
            let progress = menubar_y / menubar_height;
            self.window.setAlphaValue(1.0 - progress);
        }
    }
}
```

**Matching system animation exactly:**

Apple doesn't expose the easing curve, but we can **track the menu bar's actual position** in real-time rather than trying to replicate the animation independently:

```rust
// Frame-locked tracking via CVDisplayLink (syncs with screen refresh)
fn start_menubar_tracking(&self) {
    let display_link = CVDisplayLink::new();

    display_link.set_callback(|_| {
        // Get current menu bar height from screen geometry
        let current_offset = self.screen.frame().height
            - self.screen.visibleFrame().height
            - self.screen.visibleFrame().origin.y;

        // Move RustyBar to match — perfectly synced, no easing guesswork
        self.set_bar_offset(current_offset);

        // Stop tracking when transition complete
        if self.transition_complete(current_offset) {
            display_link.stop();
        }
    });

    display_link.start(); // Fires at display refresh rate (60/120 Hz)
}
```

This makes RustyBar **frame-locked** to the menu bar — no need to reverse-engineer the curve, it just follows the actual position every frame.

### Notch Detection

```rust
// Detect notch presence per screen
fn has_notch(screen: &NSScreen) -> bool {
    // macOS 12+ exposes safe area insets
    if #available(macOS 12.0, *) {
        let insets = screen.safeAreaInsets();
        return insets.top > 0.0;
    }
    false
}

fn notch_width(screen: &NSScreen) -> f64 {
    // Notch is ~200px on 14" MBP, ~180px on 16" MBP
    // Can also detect via auxiliaryTopLeftArea/auxiliaryTopRightArea
    let frame = screen.frame();
    let visible = screen.visibleFrame();

    // The "missing" top area indicates notch
    if has_notch(screen) {
        // Calculate from safe areas or use known values per model
        return 200.0; // Conservative default
    }
    0.0
}
```

### Per-Monitor Configuration

```toml
[bar]
height = 28
background = "#1a1a2e"
font = "JetBrains Mono"
font_size = 13

# Default layout (external monitors, no notch)
[bar.layout]
mode = "continuous"  # Single bar across full width

# Override for built-in display (with notch)
[bar.displays."built-in"]
layout.mode = "split"  # Two halves around notch
layout.notch_padding = 8  # Gap between bar edges and notch
```

### Window Creation per Layout Mode

```rust
fn create_bar_windows(screen: &NSScreen) -> Vec<Id<NSWindow>> {
    if has_notch(screen) {
        // Two windows, one on each side of notch
        let notch_w = notch_width(screen);
        let screen_w = screen.frame().width();
        let half_w = (screen_w - notch_w) / 2.0;

        vec![
            create_window(0.0, half_w),              // Left half
            create_window(half_w + notch_w, half_w), // Right half
        ]
    } else {
        // Single window spanning full width
        vec![create_window(0.0, screen.frame().width())]
    }
}
```

## Features

### Core Bar Features

- **Notch-aware layout** — Split or continuous based on display
- **Per-monitor bars** — Different configs per display
- **Four-zone module placement** — left.left, left.right, right.left, right.right
- **Transparency & blur** — Native vibrancy effects (NSVisualEffectView)
- **Animations** — Smooth transitions for state changes

### Popups: Drawer Mode vs Floating Mode

**Two approaches to expanded content:**

#### Drawer Mode (Preferred)

The bar itself extends downward like a drawer — same background, feels integrated:

```
Before click:
┌─────────────────────────────────────────────────────────┐
│ [workspace] [window title]          [cpu] [clock] [⚡] │
└─────────────────────────────────────────────────────────┘

After clicking [clock]:
┌─────────────────────────────────────────────────────────┐
│ [workspace] [window title]          [cpu] [clock] [⚡] │  ← bar unchanged
├─────────────────────────────────────────────────────────┤
│                                                         │
│              ┌─────────────────────┐                    │
│              │     January 2026    │                    │  ← drawer content
│              │  Su Mo Tu We Th Fr Sa│                   │
│              │      1  2  3  4  5  6│                    │
│              │   7  8  9 10 11 12 13│                    │
│              └─────────────────────┘                    │
│                                                         │
└─────────────────────────────────────────────────────────┘
                    ↑ slides down, same background
```

**Behavior:**
- Bar extends downward (same background color, seamless)
- Content renders inside the extended area
- Click outside → drawer retracts
- **Only one drawer open at a time**
- Clicking another drawer module → content swaps, height transitions smoothly

**Configuration split:**

```toml
# ===== APP LEVEL (bar config) =====
# Drawer behavior is global — all drawers share these settings
[bar.drawer]
transition = "slide"          # "slide", "fade", "instant"
duration = 200                # Transition duration in ms
border_behavior = "slides"    # "slides" (border at drawer bottom) or "fixed" (border stays at bar)
border_color = "#7aa2f7"
border_width = 1

# ===== MODULE LEVEL =====
# Each module only defines its content and height
[[modules.right.right]]
type = "clock"
on_click = "drawer"
drawer_content = "calendar"
drawer_height = 300           # Only height defined here

[[modules.right.left]]
type = "cpu"
on_click = "drawer"
drawer_content = "system_monitor"
drawer_height = 400           # Different height, transitions smoothly
```

**Border behavior options:**

```
border_behavior = "slides" (border moves with drawer bottom):
┌─────────────────────────────────────────────────────────┐
│ [workspace] [window]              [cpu] [clock] [⚡]    │
│                                                         │
│                    [ drawer content ]                   │
│                                                         │
└─────────────────────────────────────────────────────────┘
                          ↑ border at bottom of drawer

border_behavior = "fixed" (border stays at bar, drawer has own border):
┌─────────────────────────────────────────────────────────┐
│ [workspace] [window]              [cpu] [clock] [⚡]    │
├─────────────────────────────────────────────────────────┤  ← bar border stays
│                                                         │
│                    [ drawer content ]                   │
│                                                         │
└─────────────────────────────────────────────────────────┘  ← drawer border
```

**No side borders** — only bottom border on the drawer.

**Height transition when swapping content:**
```
Drawer open (calendar, 300px) → click [cpu] → height animates to 400px
                                              content cross-fades
```

```rust
fn handle_drawer_click(&mut self, module: &Module) {
    if self.drawer.is_open() {
        if self.drawer.source_module == module.id() {
            // Same module clicked → close drawer
            self.drawer.close();
        } else {
            // Different module → swap content, animate height
            self.drawer.swap_content(
                module.drawer_content(),
                module.drawer_height(),  // Animate to new height
            );
        }
    } else {
        // Open drawer
        self.drawer.open(module.drawer_content(), module.drawer_height());
    }
}
```

#### Floating Mode (Traditional)

Popup appears below the specific module:

```
┌─────────────────────────────────────────────────────────┐
│ [workspace] [window title]          [cpu] [clock] [⚡] │
└───────────────────────────────────────────────┬─────────┘
                                                │
                                        ┌───────▼───────┐
                                        │  January 2026 │
                                        │  ┌──────────┐ │
                                        │  │ calendar │ │
                                        │  └──────────┘ │
                                        └───────────────┘
```

```toml
[[modules.right.right]]
type = "clock"
on_click = "popup"            # Floating popup
popup_content = "calendar"
popup_position = "below"      # "below", "above"
```

### Dropdowns & Popups (SketchyBar Pain Point)

Native AppKit gives us real popup capabilities:

```
┌─────────────────────────────────────────────────────────┐
│ [workspace] [window title]          [cpu] [clock ▼] [⚡]│
└─────────────────────────────────────────────────────────┘
                                            │
                                    ┌───────▼───────┐
                                    │ ▸ January     │
                                    │   2026        │
                                    │ ┌───────────┐ │
                                    │ │Mo Tu We...│ │
                                    │ │ 1  2  3...│ │
                                    │ └───────────┘ │
                                    │ ───────────── │
                                    │ 10:30 Meeting │
                                    │ 14:00 Call    │
                                    └───────────────┘
```

**Implementation approaches:**

1. **NSPopover** — Native macOS popover, auto-positions, has arrow
2. **Child NSWindow** — Full control, any position, custom chrome
3. **NSMenu** — For simple list-style dropdowns

```rust
// Popup window for rich content
fn show_popup(anchor: NSRect, content: impl View) {
    let popup = NSWindow::initWithContentRect_styleMask_backing_defer(
        calculate_popup_frame(anchor),
        NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
        NSBackingStoreType::Buffered,
        false,
    );
    popup.setLevel(NSWindowLevel::PopUpMenu);
    popup.setContentView(content.into_nsview());
    popup.makeKeyAndOrderFront(None);
}
```

**Popup types to support:**

| Type | Use Case | Example |
|------|----------|---------|
| Calendar | Date picker, events | Click on clock |
| List | Quick actions, selections | WiFi networks |
| Detail | Expanded info | System stats breakdown |
| Custom | User-defined | Weather forecast |

**Popup capabilities (fixing SketchyBar limitations):**

```rust
// Scrollable content
fn create_scrollable_popup(content_height: f64) -> NSScrollView {
    let scroll_view = NSScrollView::new();
    scroll_view.setHasVerticalScroller(true);
    scroll_view.setAutohidesScrollers(true);  // Only show when needed
    scroll_view.setScrollerStyle(NSScrollerStyle::Overlay);  // Modern overlay scrollbars
    scroll_view
}

// Auto-close on outside interaction
fn setup_popup_auto_close(popup: &NSWindow) {
    // Option 1: Built-in behavior
    popup.setHidesOnDeactivate(true);

    // Option 2: Global click monitor for more control
    NSEvent::addGlobalMonitorForEventsMatchingMask_handler(
        NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown,
        |event| {
            if !popup.frame().contains(event.locationInWindow()) {
                popup.close();
            }
        }
    );
}

// Full font control
fn render_text(text: &str, config: &TextConfig) -> CTLine {
    let font = CTFont::new_with_name(config.font_family, config.font_size);
    let attributes = [
        (kCTFontAttributeName, font),
        (kCTForegroundColorAttributeName, config.color),
        (kCTKernAttributeName, config.letter_spacing),
    ];
    // ... full Core Text control
}
```

| Feature | Implementation |
|---------|----------------|
| Scrolling | NSScrollView with overlay scrollers |
| Auto-close | `hidesOnDeactivate` + global event monitor |
| Font sizing | Core Text with any size/weight/style |
| Custom styling | Core Graphics — shadows, gradients, borders |
| Cursor control | NSCursor push/pop per tracking area |
| Border/background | Same styling as bar modules (background, border_color, border_width, corner_radius) |

### Mouse Tracking (SketchyBar Pain Point)

Full mouse awareness via NSTrackingArea:

```rust
// Track mouse enter/exit/move for any region
fn setup_tracking(view: &NSView, module_bounds: NSRect) {
    let tracking_area = NSTrackingArea::initWithRect_options_owner_userInfo(
        module_bounds,
        NSTrackingAreaOptions::MouseEnteredAndExited |
        NSTrackingAreaOptions::MouseMoved |
        NSTrackingAreaOptions::ActiveAlways,
        view,
        None,
    );
    view.addTrackingArea(tracking_area);
}

// Global mouse position (works even outside our window)
fn get_mouse_position() -> NSPoint {
    NSEvent::mouseLocation()
}
```

**Mouse features:**

- **Hover states** — Change appearance on mouse enter/exit
- **Tooltips** — Show info on hover delay
- **Click regions** — Left/right/middle click handlers per module
- **Drag support** — Reorder modules, drag values (like volume slider)
- **Global position** — Know where mouse is even outside bar
- **Scroll events** — Scroll wheel on modules (volume, brightness)
- **Cursor control** — Pointer cursor on clickable items (like web `cursor: pointer`)

```rust
// Automatic cursor change for clickable modules
fn mouse_entered(&self, module: &Module) {
    if module.has_click_handler() {
        // Pointing hand cursor (like clicking a link)
        NSCursor::pointingHandCursor().push();
    }
}

fn mouse_exited(&self) {
    NSCursor::pop();
}
```

Any module with `on_click` defined automatically shows the pointing hand cursor on hover — no extra config needed.

```toml
# Config example
[[modules.right]]
type = "volume"
on_hover = "show_tooltip"
on_click = "toggle_mute"
on_right_click = "show_audio_devices"
on_scroll = "adjust_volume"

[[modules.right]]
type = "clock"
on_click = "show_calendar_popup"
on_hover = "show_full_date"
```

### Interaction Model

```
┌─────────────────────────────────────────────────────────┐
│                    Event Sources                         │
├──────────┬──────────┬───────────┬───────────┬──────────┤
│  Mouse   │ Keyboard │  System   │   IPC     │  Timer   │
│  Events  │ Hotkeys  │  Events   │  Socket   │  Ticks   │
└────┬─────┴────┬─────┴─────┬─────┴─────┬─────┴────┬─────┘
     │          │           │           │          │
     ▼          ▼           ▼           ▼          ▼
┌─────────────────────────────────────────────────────────┐
│                    Event Router                          │
├─────────────────────────────────────────────────────────┤
│  Dispatches to modules, manages popup lifecycle,         │
│  handles global hotkeys, debounces rapid events          │
└─────────────────────────────────────────────────────────┘
     │
     ▼
┌─────────────────────────────────────────────────────────┐
│                    Module Layer                          │
│  Each module receives relevant events, updates state,    │
│  requests redraws, can spawn popups                      │
└─────────────────────────────────────────────────────────┘
```

### Menu Bar Item Proxying

Trigger existing menu bar apps' popups from RustyBar with custom icons:

```
┌─── System Menu Bar (hidden or Bartender-collapsed) ───┐
│  [Clock.app] [OtherApp] [etc]                         │
└───────────────────────────────────────────────────────┘
         │ click forwarded
         ▼
┌─── RustyBar ──────────────────────────────────────────┐
│  [workspaces]  [window]           [🕐 custom icon]    │
└───────────────────────────────────────────────────────┘
```

**Approach 1: Click Forwarding**

```rust
// Forward click to a hidden menu bar item
fn proxy_click_to_menubar_item(bundle_id: &str) -> Result<()> {
    // Find the status item's position via Accessibility API
    let menubar = AXUIElement::system_wide();
    let status_items = menubar.children()?
        .filter(|el| el.role() == AXRole::MenuBarItem);

    let target = status_items
        .find(|item| item.bundle_id() == bundle_id)?;

    let frame = target.frame()?;

    // Inject click at that position
    let event = CGEvent::new_mouse_event(
        CGEventType::LeftMouseDown,
        frame.center(),
        CGMouseButton::Left,
    )?;
    event.post(CGEventTapLocation::HID);

    // Mouse up
    let event = CGEvent::new_mouse_event(
        CGEventType::LeftMouseUp,
        frame.center(),
        CGMouseButton::Left,
    )?;
    event.post(CGEventTapLocation::HID);

    Ok(())
}
```

**Approach 2: App-Specific Integration**

```toml
# Config for proxied items
[[modules.right]]
type = "proxy"
bundle_id = "com.apple.clock"
icon = "󰥔"                    # Custom icon
tooltip = "Clock"
# The popup comes from the original app, but triggered from here
```

For apps with APIs:

```toml
[[modules.right]]
type = "proxy"
name = "Fantastical"
trigger = "url"
url = "x-fantastical3://show"
icon = "󰃭"

[[modules.right]]
type = "proxy"
name = "1Password"
trigger = "applescript"
script = 'tell application "1Password" to activate'
icon = "󰌾"
```

**Limitations:**

- Popup appears at original menu bar location (top of screen)
- Requires accessibility permissions
- Some apps may not respond to synthetic clicks
- Can't restyle the app's popup itself (it's their window)

**Workaround for popup location:**

Hide the system menu bar entirely (`defaults write NSGlobalDomain _HIHideMenuBar -bool true` + restart), then RustyBar becomes the only bar and click forwarding puts the popup near the top where expected anyway.

### Keyboard Shortcuts

Global hotkeys for bar interaction:

```toml
[hotkeys]
toggle_bar = "ctrl+alt+b"
focus_bar = "ctrl+alt+f"  # Navigate modules with arrow keys
reload_config = "ctrl+alt+r"
```

### Widget Gallery (Popup Content)

Pre-built popup widgets:

- **Calendar** — Month view with event integration (CalendarKit or manual)
- **System monitor** — CPU/RAM/disk graphs
- **Audio mixer** — Per-app volume controls
- **WiFi picker** — Available networks
- **Bluetooth** — Connected devices
- **Now playing** — Media controls + album art
- **Weather** — Forecast (requires API key)
- **Clipboard history** — Recent clipboard items
- **Quick notes** — Jot something down

### Theming

```toml
[theme]
name = "tokyo-night"

[theme.colors]
background = "#1a1b26"
foreground = "#c0caf5"
accent = "#7aa2f7"
warning = "#e0af68"
error = "#f7768e"
success = "#9ece6a"

[theme.effects]
blur = true
blur_radius = 20
vibrancy = "dark"  # light, dark, ultra_dark
corner_radius = 8
border_width = 1
border_color = "#292e42"
```

## Architecture

```
┌───────────────────────────────────────────────────────────────┐
│                        RustyBar Core                          │
├───────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │
│  │   Config    │  │   Layout    │  │      Modules        │   │
│  │   Loader    │──│   Engine    │──│  (workspace, clock, │   │
│  │   (TOML)    │  │ (4-zone)    │  │   battery, etc.)    │   │
│  └─────────────┘  └─────────────┘  └─────────────────────┘   │
│         │               │                    │                │
│         ▼               ▼                    ▼                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                   Window Manager                         │ │
│  │  • Notch detection          • Per-monitor windows       │ │
│  │  • Split/continuous mode    • Window level management   │ │
│  └─────────────────────────────────────────────────────────┘ │
│         │               │                    │                │
│         ▼               ▼                    ▼                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │
│  │   Render    │  │   Event     │  │      Popup          │   │
│  │   Engine    │  │   System    │  │      Manager        │   │
│  └─────────────┘  └─────────────┘  └─────────────────────┘   │
├───────────────────────────────────────────────────────────────┤
│                      macOS Bindings                           │
│     objc2 · objc2-app-kit · core-graphics · core-text         │
└───────────────────────────────────────────────────────────────┘
```

### Layout Engine

```rust
struct BarLayout {
    left_half: HalfLayout,
    right_half: HalfLayout,
    notch_width: f64,  // 0 if no notch
}

struct HalfLayout {
    left_modules: Vec<ModuleInstance>,   // Align to outer edge
    right_modules: Vec<ModuleInstance>,  // Align to inner edge (toward center/notch)
}

impl BarLayout {
    fn calculate_positions(&self, screen_width: f64) -> ModulePositions {
        let half_width = if self.notch_width > 0.0 {
            (screen_width - self.notch_width) / 2.0
        } else {
            screen_width / 2.0
        };

        // Left half: modules.left.left from x=0 →, modules.left.right from x=half_width ←
        // Right half: modules.right.left from x=half_width+notch →, modules.right.right from x=screen_width ←
        // ...
    }
}
```

### Core Components

1. **Window Manager** — Creates and positions the bar window
   - NSWindow with `level: .statusBar` or custom level
   - Frameless, transparent background
   - Handles display changes (multiple monitors)

2. **Event System** — Hooks into macOS events
   - Workspace/Space changes (CGSConnection private APIs or Accessibility)
   - Window focus changes
   - Application launch/quit
   - System events (battery, wifi, audio volume)

3. **Config Loader** — Parses user configuration
   - TOML for static config
   - Optional Lua/mlua for dynamic scripting
   - File watcher for hot-reload

4. **Render Engine** — Draws bar content
   - Core Graphics for drawing
   - Core Text for text rendering
   - Efficient dirty-rect invalidation

## Key Crates

| Crate | Purpose |
|-------|---------|
| `objc2` | Modern Objective-C runtime bindings |
| `objc2-app-kit` | AppKit bindings (NSWindow, NSView, etc.) |
| `objc2-foundation` | Foundation types (NSString, NSArray, etc.) |
| `core-graphics` | Drawing primitives |
| `core-text` | Text layout and rendering |
| `rhai` | Scripting engine |
| `toml` | Configuration parsing |
| `notify` | File system watching for hot-reload |
| `tokio` | Async runtime for event handling |

## Window Setup

```rust
// Pseudocode for window creation
use objc2_app_kit::{NSWindow, NSWindowLevel, NSBackingStoreType};
use objc2_foundation::NSRect;

fn create_bar_window() -> Id<NSWindow> {
    let frame = NSRect::new(
        NSPoint::new(0.0, screen_height - BAR_HEIGHT),
        NSSize::new(screen_width, BAR_HEIGHT),
    );

    let window = NSWindow::initWithContentRect_styleMask_backing_defer(
        frame,
        NSWindowStyleMask::Borderless,
        NSBackingStoreType::Buffered,
        false,
    );

    window.setLevel(NSWindowLevel::StatusBar);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces |
        NSWindowCollectionBehavior::Stationary |
        NSWindowCollectionBehavior::IgnoresCycle
    );
    window.setBackgroundColor(NSColor::clearColor());
    window.setOpaque(false);
    window.setHasShadow(false);

    window
}
```

## Event Hooks

### Space/Desktop Changes

Two approaches:

1. **Private CGSConnection APIs** (what SketchyBar uses)
   - `CGSRegisterNotifyProc` for space change notifications
   - Requires linking against private SkyLight framework
   - Most reliable but uses undocumented APIs

2. **Accessibility API**
   - `AXObserver` for window/app notifications
   - More public but less comprehensive
   - Requires accessibility permissions

### System Stats

```rust
// Battery via IOKit
// WiFi via CoreWLAN
// Audio via CoreAudio
// CPU/Memory via mach APIs
```

## Module System

Each bar segment is a module with flexible rendering:

### Module Output Structure

```rust
struct ModuleOutput {
    // Content (all optional, combine as needed)
    icon: Option<Icon>,           // Nerd Font icon or image
    header: Option<String>,       // Small text above (e.g., "RAM", "CPU")
    value: Option<String>,        // Main text (e.g., "67%", "4:46 PM")

    // Styling (default state)
    foreground: Option<Color>,
    background: Option<Color>,    // Fill color (None = transparent)
    border_color: Option<Color>,  // Border color (None = no border)
    border_width: Option<f64>,    // Border thickness in pixels
    corner_radius: Option<f64>,   // 0 = square, >0 = rounded
    icon_color: Option<Color>,
    header_color: Option<Color>,

    // Hover state (all optional, only specified props change on hover)
    hover: Option<HoverStyle>,
}

struct HoverStyle {
    foreground: Option<Color>,    // Value text color on hover
    background: Option<Color>,
    border_color: Option<Color>,
    icon_color: Option<Color>,
    header_color: Option<Color>,
    transition_ms: Option<u32>,   // Override default transition duration
}

struct LoadingStyle {
    icon: Option<Icon>,           // Spinner or loading icon
    text: Option<String>,         // "Loading..." or similar
    foreground: Option<Color>,
    animation: LoadingAnimation,  // Spinner, pulse, etc.
}

enum LoadingAnimation {
    Spinner,      // Rotating icon
    Pulse,        // Fade in/out
    Dots,         // Animated "..."
    None,         // Static loading state
}

    // Layout hints
    min_width: Option<f64>,
    padding: Option<Padding>,
}
```

### Rendering Layouts

Based on your screenshots, modules can render in different layouts:

```
Icon + Value (simple):
┌─────────────┐
│  󰕾  50%    │   ← volume
└─────────────┘

Icon only:
┌───┐
│ 󰤨 │   ← wifi
└───┘

Header + Value (stacked):
┌───────┐
│  RAM  │   ← small header
│  67%  │   ← large value
└───────┘

Header + Value + Icon:
┌─────────────┐
│ Thu Jan 29  │   ← small header
│ 4:46 PM  󰃭 │   ← large value + icon
└─────────────┘

Badge (any content, configurable corners):
┌─────┐
│ [T] │   ← single char, rounded
└─────┘

┌───────────┐
│ 3 updates │   ← multiple chars, rounded
└───────────┘

┼─────┼
│ REC │   ← square corners (radius: 0)
┼─────┼
```

### Module Trait (Full API)

```rust
trait Module: Send + Sync {
    /// Unique identifier for this module instance
    fn id(&self) -> &str;

    /// Human-readable name (for logging/debugging)
    fn name(&self) -> &str;

    /// Called on interval or event to get current state
    fn update(&mut self, ctx: &UpdateContext) -> ModuleOutput;

    /// Poll interval, None = event-driven only
    fn interval(&self) -> Option<Duration> { None }

    /// Handle click events
    fn on_click(&mut self, event: ClickEvent) {}

    /// Handle scroll events
    fn on_scroll(&mut self, event: ScrollEvent) {}

    /// Handle hover enter
    fn on_hover_enter(&mut self) {}

    /// Handle hover exit
    fn on_hover_exit(&mut self) {}

    /// Events this module subscribes to (space change, window focus, etc.)
    fn subscriptions(&self) -> Vec<EventType> { vec![] }
}

/// Context provided to module on each update
struct UpdateContext {
    /// Current time
    now: DateTime<Local>,
    /// Screen this bar is on
    screen: ScreenInfo,
    /// Whether bar is in debug mode
    debug: bool,
    /// Access to system APIs
    system: SystemApi,
}

/// System APIs available to modules
struct SystemApi {
    fn battery(&self) -> BatteryInfo;
    fn cpu(&self) -> CpuInfo;
    fn memory(&self) -> MemoryInfo;
    fn wifi(&self) -> WifiInfo;
    fn volume(&self) -> VolumeInfo;
    fn active_window(&self) -> WindowInfo;
    fn active_space(&self) -> SpaceInfo;
    fn run_command(&self, cmd: &str) -> Result<String>;
    fn http_get(&self, url: &str) -> Result<String>;  // For Rhai scripts
}

/// Click event details
struct ClickEvent {
    button: MouseButton,      // Left, Right, Middle
    position: Point,          // Click position within module
    modifiers: Modifiers,     // Shift, Ctrl, Alt, Cmd held
}

/// Scroll event details
struct ScrollEvent {
    delta: f64,               // Positive = up/right, negative = down/left
    direction: ScrollDirection,
    modifiers: Modifiers,
}

/// Events modules can subscribe to
enum EventType {
    SpaceChanged,
    WindowFocused,
    AppLaunched,
    AppQuit,
    DisplayChanged,
    VolumeChanged,
    WifiChanged,
    BatteryChanged,
    PowerStateChanged,        // Plugged in / on battery
}
```

### Module Output

The module returns its complete state on each update, including all visual states:

```rust
struct ModuleOutput {
    // ===== Visibility =====
    visible: bool,                // false = module hidden entirely

    // ===== Content =====
    icon: Option<Icon>,
    header: Option<String>,
    value: Option<String>,

    // ===== Layout =====
    layout: ModuleLayout,         // Inline, Stacked, IconOnly, Badge
    min_width: Option<f64>,
    max_width: Option<f64>,
    flex: bool,
    margin_left: Option<f64>,
    margin_right: Option<f64>,

    // ===== Appearance (all states defined by module) =====
    appearance: ModuleAppearance,
}

/// Complete appearance definition — module owns all its visual states
struct ModuleAppearance {
    // Default state
    default: StyleState,

    // Hover state (mouse over)
    hover: Option<StyleState>,

    // Active state (mouse down, or "on" state like VPN connected)
    active: Option<StyleState>,

    // Loading state (fetching data)
    loading: Option<LoadingState>,

    // Error state (module failed)
    error: Option<StyleState>,
}

/// Visual properties for a single state
struct StyleState {
    icon: Option<Icon>,
    icon_color: Option<Color>,
    foreground: Option<Color>,      // Value text
    header_color: Option<Color>,
    background: Option<Color>,
    border_color: Option<Color>,
    border_width: Option<f64>,
    corner_radius: Option<f64>,
}

/// Loading state with animation
struct LoadingState {
    icon: Option<Icon>,
    text: Option<String>,
    foreground: Option<Color>,
    background: Option<Color>,
    animation: LoadingAnimation,    // Spinner, Pulse, Dots, None
}

enum LoadingAnimation {
    Spinner,      // Rotating icon
    Pulse,        // Fade in/out
    Dots,         // Animated "..."
    None,         // Static
}

/// Current module state (set by module logic)
enum ModuleState {
    Default,
    Hover,        // Set by bar when mouse enters
    Active,       // Set by module (e.g., VPN connected, recording, etc.)
    Loading,      // Set by module while fetching
    Error,        // Set by module on failure
}

impl ModuleOutput {
    fn hidden() -> Self {
        Self { visible: false, ..Default::default() }
    }

    fn loading() -> Self {
        Self {
            visible: true,
            state: ModuleState::Loading,
            ..Default::default()
        }
    }

    fn error(message: &str) -> Self {
        Self {
            visible: true,
            state: ModuleState::Error,
            value: Some(message.into()),
            ..Default::default()
        }
    }
}
```

### Module Defines All States

The module is fully self-contained — it defines its own icon, appearance, and all states:

```rust
impl Module for BatteryModule {
    fn update(&mut self, ctx: &UpdateContext) -> ModuleOutput {
        let battery = ctx.system.battery();

        if battery.is_charging && self.config.hide_when_charging {
            return ModuleOutput::hidden();
        }

        let icon = self.icon_for_level(battery.percentage, battery.is_charging);
        let is_low = battery.percentage < 20;

        ModuleOutput {
            visible: true,
            icon: Some(icon),
            value: Some(format!("{}%", battery.percentage)),
            layout: ModuleLayout::Inline,

            appearance: ModuleAppearance {
                default: StyleState {
                    icon: Some(icon),
                    icon_color: Some(if is_low { Color::WARNING } else { Color::NORMAL }),
                    foreground: Some(Color::TEXT),
                    background: None,
                    ..Default::default()
                },
                hover: Some(StyleState {
                    background: Some(Color::HOVER_BG),
                    foreground: Some(Color::TEXT_BRIGHT),
                    ..Default::default()
                }),
                active: None,  // Battery doesn't have an "active" state
                loading: None, // Battery doesn't load
                error: Some(StyleState {
                    icon: Some(Icon::nerd("")),
                    icon_color: Some(Color::ERROR),
                    foreground: Some(Color::ERROR),
                    ..Default::default()
                }),
            },

            ..Default::default()
        }
    }
}
```

### VPN Module (with Active State)

```rust
impl Module for VpnModule {
    fn update(&mut self, ctx: &UpdateContext) -> ModuleOutput {
        let vpn = ctx.system.vpn();

        ModuleOutput {
            visible: true,
            state: if vpn.connected { ModuleState::Active } else { ModuleState::Default },
            icon: Some(Icon::nerd("󰖂")),
            value: if vpn.connected { Some(vpn.name.clone()) } else { None },

            appearance: ModuleAppearance {
                default: StyleState {
                    icon_color: Some(Color::MUTED),  // Gray when disconnected
                    ..Default::default()
                },
                hover: Some(StyleState {
                    icon_color: Some(Color::TEXT),
                    background: Some(Color::HOVER_BG),
                    ..Default::default()
                }),
                active: Some(StyleState {
                    icon_color: Some(Color::SUCCESS),  // Green when connected
                    foreground: Some(Color::SUCCESS),
                    ..Default::default()
                }),
                loading: Some(LoadingState {
                    icon: Some(Icon::nerd("󰖂")),
                    animation: LoadingAnimation::Pulse,
                    foreground: Some(Color::MUTED),
                    ..Default::default()
                }),
                error: Some(StyleState {
                    icon_color: Some(Color::ERROR),
                    ..Default::default()
                }),
            },

            ..Default::default()
        }
    }
}
```

### Weather Module (with Loading State)

```rust
impl Module for WeatherModule {
    fn update(&mut self, ctx: &UpdateContext) -> ModuleOutput {
        // If fetching, show loading state
        if self.is_fetching {
            return ModuleOutput {
                visible: true,
                state: ModuleState::Loading,
                appearance: self.appearance(),
                ..Default::default()
            };
        }

        // If fetch failed, show error
        if let Some(err) = &self.last_error {
            return ModuleOutput::error(&err);
        }

        // Normal display
        let weather = self.cached_weather.as_ref().unwrap();
        ModuleOutput {
            visible: true,
            icon: Some(self.icon_for_condition(&weather.condition)),
            value: Some(format!("{}°", weather.temp)),
            appearance: self.appearance(),
            ..Default::default()
        }
    }

    fn appearance(&self) -> ModuleAppearance {
        ModuleAppearance {
            default: StyleState { /* ... */ },
            hover: Some(StyleState { /* ... */ }),
            active: None,
            loading: Some(LoadingState {
                icon: Some(Icon::nerd("󰇚")),
                text: Some("...".into()),
                animation: LoadingAnimation::Dots,
                foreground: Some(Color::MUTED),
                ..Default::default()
            }),
            error: Some(StyleState {
                icon: Some(Icon::nerd("")),
                icon_color: Some(Color::ERROR),
                ..Default::default()
            }),
        }
    }
}
```
```

### Conditional Visibility Examples

**Battery: only show when not plugged in**
```rust
fn update(&mut self, ctx: &UpdateContext) -> ModuleOutput {
    let battery = ctx.system.battery();

    if battery.is_charging {
        return ModuleOutput::hidden();
    }

    ModuleOutput {
        visible: true,
        icon: Some(self.icon_for_level(battery.percentage)),
        value: Some(format!("{}%", battery.percentage)),
        ..Default::default()
    }
}
```

**Volume: only show when muted or below threshold**
```rust
fn update(&mut self, ctx: &UpdateContext) -> ModuleOutput {
    let volume = ctx.system.volume();

    if !volume.muted && volume.level > 0.1 {
        return ModuleOutput::hidden();
    }

    ModuleOutput {
        visible: true,
        icon: Some(if volume.muted { "󰖁" } else { "󰕿" }.into()),
        value: Some(format!("{}%", (volume.level * 100.0) as u8)),
        ..Default::default()
    }
}
```

**Notification badge: only show when count > 0**
```rust
fn update(&mut self, ctx: &UpdateContext) -> ModuleOutput {
    let count = self.notification_count;

    if count == 0 {
        return ModuleOutput::hidden();
    }

    ModuleOutput {
        visible: true,
        value: Some(count.to_string()),
        background: Some(Color::from_hex("#f7768e")),
        corner_radius: Some(8.0),
        ..Default::default()
    }
}
```

**Rhai script: conditional visibility**
```rhai
fn update(ctx) {
    let battery = ctx.system.battery();

    if battery.is_charging {
        return #{ visible: false };
    }

    #{
        visible: true,
        icon: battery_icon(battery.percentage),
        value: `${battery.percentage}%`
    }
}
```

### Config-based Conditional Visibility

For simple conditions without scripting:

```toml
[[modules.right.right]]
type = "battery"
show_when = "not charging"       # Built-in condition

[[modules.right.right]]
type = "volume"
show_when = "muted or level < 10"

[[modules.right.right]]
type = "wifi"
show_when = "disconnected"       # Only show when there's a problem

[[modules.right.right]]
type = "vpn"
show_when = "connected"          # Only show when VPN is active
```

### Text Rendering with Core Text

```rust
struct TextStyle {
    font_family: String,
    font_size: f64,
    font_weight: FontWeight,
    color: Color,
    letter_spacing: f64,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "JetBrains Mono".into(),
            font_size: 13.0,
            font_weight: FontWeight::Regular,
            color: Color::WHITE,
            letter_spacing: 0.0,
        }
    }
}

// Two text sizes for header/value layout
struct ModuleTextStyles {
    header: TextStyle,  // e.g., size 9, weight light, muted color
    value: TextStyle,   // e.g., size 14, weight medium, bright color
    icon: TextStyle,    // Nerd Font at appropriate size
}
```

### Nerd Fonts

Icons via Nerd Fonts (user should have a patched font installed):

```toml
[bar]
font = "JetBrainsMono Nerd Font"  # or "Hack Nerd Font", etc.
icon_font = "JetBrainsMono Nerd Font"  # Can be separate

# Icons are just Unicode codepoints
[[modules.right.right]]
type = "battery"
icons = ["󰁺", "󰁻", "󰁼", "󰁽", "󰁾", "󰁿", "󰂀", "󰂁", "󰂂", "󰁹"]  # 0-100%
charging_icon = "󰂄"
```

**Fallback behavior:** If Nerd Font icons don't render (font not installed), modules show fallback text:

| Icon | Fallback |
|------|----------|
| 󰁹 | `[bat]` |
| 󰕾 | `[vol]` |
| 󰤨 | `[wifi]` |
| 󰻠 | `[cpu]` |
| 󰍛 | `[mem]` |

```rust
fn render_icon(&self, icon: &Icon) -> RenderedContent {
    if self.can_render_icon(icon) {
        RenderedContent::Icon(icon.clone())
    } else {
        RenderedContent::Text(icon.fallback_text())
    }
}
```

### Built-in Modules

- `workspace` — Current/all spaces (integrates with Aerospace)
- `window_title` — Focused window title
- `app_icon` — Focused app icon
- `clock` — Date/time with format string
- `battery` — Battery percentage and charging state
- `wifi` — Network name and signal strength
- `volume` — Audio output volume
- `cpu` — CPU usage percentage
- `memory` — RAM usage
- `disk` — Disk usage percentage
- `script` — Custom Rhai script module
- `separator` — Visual divider between modules
- `static` — Static text/icon/badge

### Separators

Visual dividers between modules:

```toml
[[modules.right.left]]
type = "separator"
style = "line"         # Vertical pipe
color = "#565f89"
width = 1
height = 12            # Shorter than bar height for visual balance

[[modules.right.left]]
type = "separator"
style = "dot"
color = "#565f89"
size = 4

[[modules.right.left]]
type = "separator"
style = "icon"
icon = "│"             # Or any Nerd Font icon: "", "◆", etc.
color = "#565f89"

[[modules.right.left]]
type = "separator"
style = "space"        # Just empty space
width = 16
```

### Module Grouping

Multiple modules can share a background:

```toml
[[modules.right.left]]
type = "group"
background = "#292e42"
corner_radius = 6
padding = 4
modules = ["cpu", "memory", "disk"]  # References to module IDs

# Or inline:
[[modules.right.left]]
type = "group"
background = "#292e42"
corner_radius = 6

[[modules.right.left.modules]]
type = "cpu"
id = "cpu"
layout = "stacked"
header = "CPU"

[[modules.right.left.modules]]
type = "separator"
style = "line"
color = "#565f89"

[[modules.right.left.modules]]
type = "memory"
id = "memory"
layout = "stacked"
header = "RAM"
```

```
┌──────────────────────────────┐
│ CPU  │  RAM  │  DISK         │  ← shared background, separators inside
│ 15%  │  67%  │  42%          │
└──────────────────────────────┘
```

### Module Spacing

Horizontal margin per module (vertical alignment stays consistent):

```toml
[[modules.right.right]]
type = "clock"
margin_left = 8        # Extra space before this module
margin_right = 4       # Extra space after this module
```

## File Locations

```
~/.config/rustybar/
├── config.toml           # Main configuration
├── scripts/              # Custom Rhai modules
│   ├── weather.rhai
│   └── github-notifications.rhai
└── rustybar.log          # Debug log (when --debug enabled)
```

Or log in: `~/.local/share/rustybar/rustybar.log`

## First Run & Config Errors

**First run (no config exists):**
- Generate default `~/.config/rustybar/config.toml` with sensible starter setup
- Default includes: clock, battery, cpu, workspace modules
- User can customize from there

**Config errors (syntax error, invalid value):**
- Show red error banner in the bar itself
- Banner shows brief error message, clickable to open config
- Log full error details to log file

```
┌─────────────────────────────────────────────────────────┐
│ ⚠ Config error: invalid color at line 42 (click to fix)│
└─────────────────────────────────────────────────────────┘
```

## Colors

Hex format with transparency support (8-character hex):

```toml
background = "#1a1a2e"        # Opaque (RGB)
background = "#1a1a2eff"      # Opaque (RGBA, ff = 100%)
background = "#1a1a2e80"      # 50% transparent
background = "#ffffff00"      # Fully transparent
```

Format: `#RRGGBB` or `#RRGGBBAA`

## Animation Easing

All transitions use **ease-in-out** curve:
- Hover state changes
- Drawer open/close
- Height transitions
- Content width changes
- Module appear/disappear

```rust
fn ease_in_out(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}
```

## Configuration (TOML)

```toml
[bar]
height = "auto"        # Match system menu bar height, or specify pixels (e.g., 28)
background = "#1a1a2e"
foreground = "#c0caf5"
font = "JetBrains Mono"
font_size = 13
padding = 8
transition_ms = 150    # Default hover transition duration (0 = instant)

# Notch handling
[bar.notch]
mode = "split"         # Always split into two halves
gap = 8                # Minimum gap between halves (when no fake notch)

# Fake notch (only applies to external monitors without real notch)
[bar.notch.fake]
enabled = true
width = 200            # Width of the notch
color = "#000000"      # Notch fill color (match bezel or customize)
corner_radius = 8      # Curves at bottom corners where notch meets bar

# Launch on startup
[bar.startup]
launch_on_login = true    # Install/remove LaunchAgent automatically

# ┌─────────────────────┐         ┌─────────────────────┐
# │ LEFT.LEFT  LEFT.RIGHT│ [NOTCH] │RIGHT.LEFT RIGHT.RIGHT│
# └─────────────────────┘         └─────────────────────┘

# Left half, left-aligned (grows →)
[[modules.left.left]]
type = "workspace"
icons = ["󰎤", "󰎧", "󰎪", "󰎭", "󰎱", "󰎳", "󰎶", "󰎹"]
active_color = "#7aa2f7"
on_click = "Aerospace -m space --focus {index}"

[[modules.left.left]]
type = "app_icon"

# Left half, right-aligned (grows ←)
[[modules.left.right]]
type = "window_title"
max_length = 40
truncate = "middle"

# Right half, left-aligned (grows →)
# Header/value layout for system stats
[[modules.right.left]]
type = "cpu"
layout = "stacked"     # header above value
header = "CPU"
format = "{percentage}%"
interval = 2000

[[modules.right.left]]
type = "memory"
layout = "stacked"
header = "RAM"
format = "{percentage}%"
interval = 5000

[[modules.right.left]]
type = "disk"
layout = "stacked"
header = "DISK"
format = "{percentage}%"
path = "/"
interval = 30000

# Right half, right-aligned (grows ←)
# Icon + value layout
[[modules.right.right]]
type = "volume"
layout = "inline"      # icon + value on same line
icon_muted = "󰖁"
icon_low = "󰕿"
icon_high = "󰕾"
format = "{percentage}%"

[[modules.right.right]]
type = "battery"
layout = "inline"
icons = ["󰁺", "󰁻", "󰁼", "󰁽", "󰁾", "󰁿", "󰂀", "󰂁", "󰂂", "󰁹"]
charging_icon = "󰂄"
format = "{percentage}%"
icon_color = "#e0af68"  # Yellow like your screenshot

[[modules.right.right]]
type = "wifi"
layout = "icon"        # Icon only
icon_connected = "󰤨"
icon_disconnected = "󰤭"
icon_color = "#7aa2f7"  # Blue/purple

# Stacked date/time
[[modules.right.right]]
type = "clock"
layout = "stacked"
header = "%a %b %d"    # "Thu Jan 29"
format = "%I:%M %p"    # "4:46 PM"
on_click = "open -a 'TheClock'"

# Badge with hover state
[[modules.right.right]]
type = "static"
layout = "badge"
text = "T"
background = "#4a4a5e"
foreground = "#c0caf5"
corner_radius = 6
on_click = "open -a 'TheClock'"

[modules.right.right.hover]
background = "#5a5a7e"      # Lighter on hover
foreground = "#ffffff"      # Brighter text

# Border that fills on hover
[[modules.right.right]]
type = "static"
layout = "badge"
text = "3 new"
border_color = "#7aa2f7"
border_width = 1
corner_radius = 4
foreground = "#7aa2f7"

[modules.right.right.hover]
background = "#7aa2f7"      # Fill appears on hover
foreground = "#1a1a2e"      # Invert text color
border_color = "#7aa2f7"

# Icon color change on hover
[[modules.right.right]]
type = "wifi"
layout = "icon"
icon_connected = "󰤨"
icon_color = "#7aa2f7"

[modules.right.right.hover]
icon_color = "#9ece6a"      # Green on hover

# Full hover transformation
[[modules.right.right]]
type = "clock"
layout = "stacked"
header = "%a %b %d"
format = "%I:%M %p"
foreground = "#c0caf5"
header_color = "#565f89"
on_click = "open -a 'TheClock'"

[modules.right.right.hover]
foreground = "#ffffff"      # Brighter value
header_color = "#7aa2f7"    # Accent header
background = "#292e42"      # Subtle highlight

# Module that fetches from external source (e.g., weather, API)
[[modules.right.left]]
type = "script"
script = "weather.rhai"
interval = 300000           # 5 minutes

[modules.right.left.loading]
icon = "󰇚"                  # Or use animation = "spinner"
text = "..."
animation = "pulse"         # "spinner", "pulse", "dots", "none"

# Optional: proxy TheClock app
# [[modules.right.right]]
# type = "proxy"
# bundle_id = "com.mschrage.TheClock"
# icon = "󰥔"
```

### Per-Display Overrides

```toml
# External monitor (no notch) - can use different layout
[displays."LG UltraFine"]
bar.notch.mode = "ignore"  # No notch, continuous bar

# Or specify modules differently per display
# [displays."LG UltraFine".modules.left.left]
# ...
```

## Error Handling

Each module has its own error boundary — a failing module doesn't crash the bar:

```rust
impl ModuleRunner {
    fn update_module(&self, module: &mut dyn Module) -> ModuleOutput {
        match std::panic::catch_unwind(|| module.update(&self.ctx)) {
            Ok(output) => output,
            Err(e) => {
                log::error!("Module '{}' panicked: {:?}", module.name(), e);
                ModuleOutput::error(module.name())  // Shows error indicator
            }
        }
    }
}
```

For Rhai scripts:
```rust
fn run_script(&self, script: &str) -> Result<ModuleOutput, RhaiError> {
    self.engine
        .eval::<ModuleOutput>(script)
        .map_err(|e| {
            log::error!("Script error: {}", e);
            e
        })
}
```

Failed modules display an error state (red icon, "error" text) rather than disappearing or crashing the bar.

## Logging & Debug Mode

```bash
# Normal mode (errors only)
rustybar

# Debug mode (verbose logging to file)
rustybar --debug

# Log file location
~/.local/share/rustybar/rustybar.log
```

```rust
fn setup_logging(debug: bool) {
    let level = if debug { LevelFilter::Debug } else { LevelFilter::Error };

    fern::Dispatch::new()
        .level(level)
        .chain(fern::log_file("~/.local/share/rustybar/rustybar.log")?)
        .apply()?;
}
```

During development, `--debug` should always be on. Log format designed for LLM consumption:
```
[2026-01-29 16:52:03] DEBUG module:cpu - Update triggered, value: 15%
[2026-01-29 16:52:03] DEBUG layout - Recalculating positions, flex modules: 2
[2026-01-29 16:52:03] ERROR module:weather - Script failed: HTTP timeout
```

## Right-Click Context Menu

Right-clicking the bar (not a module) shows a context menu:

```
┌─────────────────────┐
│ Reload Config       │
│ Edit Config...      │  → opens config file in $EDITOR
│ ─────────────────── │
│ Debug Mode ✓        │  → toggle logging
│ ─────────────────── │
│ Quit RustyBar       │
└─────────────────────┘
```

```rust
fn show_context_menu(&self, position: NSPoint) {
    let menu = NSMenu::new();
    menu.addItem(NSMenuItem::new("Reload Config", sel!(reloadConfig:)));
    menu.addItem(NSMenuItem::new("Edit Config...", sel!(editConfig:)));
    menu.addItem(NSMenuItem::separatorItem());
    menu.addItem(NSMenuItem::new("Debug Mode", sel!(toggleDebug:)));
    menu.addItem(NSMenuItem::separatorItem());
    menu.addItem(NSMenuItem::new("Quit RustyBar", sel!(quit:)));

    NSMenu::popUpContextMenu_withEvent_forView(menu, event, self.view);
}
```

## Module IDs

Modules can have explicit IDs, or get auto-generated ones:

```toml
# Explicit ID
[[modules.right.left]]
id = "main-cpu"           # You choose
type = "cpu"

# Auto-generated ID (type + index)
[[modules.right.left]]
type = "memory"           # Gets ID "memory-0"

[[modules.right.right]]
type = "clock"            # Gets ID "clock-0"

[[modules.right.right]]
type = "clock"            # Gets ID "clock-1" (second clock)
```

Auto-generated format: `{type}-{index}` where index is per-type occurrence.

Use explicit IDs when you want stable references for IPC or scripts.

## IPC / Control

Unix socket for external control (`/tmp/rustybar.sock`):

```bash
# Query state
rustybar query modules              # List all modules with IDs
rustybar query module main-cpu      # Get specific module state

# Control modules (by ID)
rustybar set main-cpu.visible false
rustybar set clock-0.value "Custom Text"

# Trigger updates
rustybar update main-cpu            # Force refresh
rustybar reload                     # Reload config

# Debug
rustybar debug                      # Dump current state
```

**Protocol (JSON over Unix socket):**
```json
// Request
{"command": "set", "module": "main-cpu", "property": "visible", "value": false}

// Response
{"ok": true}

// Query
{"command": "query", "target": "modules"}

// Response
{"ok": true, "modules": [{"id": "main-cpu", "type": "cpu", "visible": true}, ...]}
```

## Yabai Integration

Query Aerospace for space/window info:

```rust
// Via Aerospace's socket
fn query_Aerospace<T: DeserializeOwned>(command: &str) -> Result<T> {
    let socket_path = "/tmp/Aerospace_$USER.socket";
    // Send command, parse JSON response
}

// Or via shell (slower but simpler)
fn query_Aerospace_shell(args: &[&str]) -> Result<String> {
    Command::new("Aerospace").args(args).output()
}
```

## Scripting Language Options

TOML handles static config (layout, colors, fonts). Scripting handles dynamic logic (what to display, how to react to events, custom modules).

| Language | Crate | Pros | Cons |
|----------|-------|------|------|
| **Rhai** ✓ | `rhai` | Rust-native, JS-like syntax, safe sandboxing, no C deps, hot-reload | Less known, smaller community |
| Lua | `mlua` | Battle-tested, fast, huge ecosystem | Requires Lua lib, 1-indexed arrays |
| Starlark | `starlark` | Python-like, deterministic | Limited features by design |
| JavaScript | `rquickjs` | Familiar syntax | Heavier, QuickJS quirks |

### Choice: Rhai

Rhai is purpose-built for embedding in Rust. No external dependencies, safe by default, and the syntax is approachable:

```rhai
// Custom module: system stats with header/value layout
fn render() {
    let cpu = system::cpu_percent();

    #{
        header: "CPU",
        value: `${cpu}%`,
        icon: "󰻠",
        color: if cpu > 80 { colors::warning } else { colors::foreground }
    }
}

fn on_click(button) {
    if button == "left" {
        shell("open -a 'Activity Monitor'");
    }
}
```

Lua is the safe choice if you want maximum ecosystem/familiarity, but Rhai integrates more cleanly with Rust.

## Rust Tooling

Modern Rust development setup:

| Tool | Purpose |
|------|---------|
| `rustup` | Version/toolchain manager |
| `cargo` | Package manager + build (no alternatives needed) |
| `rustfmt` | Formatting (built-in via `cargo fmt`) |
| `clippy` | Linting (built-in via `cargo clippy`) |
| `cargo-nextest` | Faster test runner |
| `bacon` | Background checker (watches + runs clippy/tests) |
| `taplo` | TOML formatting for Cargo.toml |
| `cargo-machete` | Find unused dependencies |

```toml
# .cargo/config.toml - faster linking on macOS
[target.aarch64-apple-darwin]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[alias]
t = "nextest run"
c = "clippy --all-targets"
```

## Build & Distribution

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Bundle as .app (optional, for Gatekeeper)
cargo bundle --release
```

## Permissions

RustyBar needs certain macOS permissions. On first launch (or when a feature requires it), prompt the user with guidance:

```rust
fn check_permissions(&self) -> Vec<PermissionRequest> {
    let mut needed = vec![];

    // Accessibility - needed for window title, active app tracking
    if !accessibility_enabled() {
        needed.push(PermissionRequest::Accessibility {
            reason: "Track active window and app for window_title module",
            guide: "System Settings → Privacy & Security → Accessibility → Enable RustyBar",
        });
    }

    // Screen Recording - needed for some window info (optional)
    // Not always required, depends on features used

    needed
}

fn prompt_for_permission(&self, request: &PermissionRequest) {
    // Show native alert with explanation and "Open System Settings" button
    let alert = NSAlert::new();
    alert.setMessageText("Permission Required");
    alert.setInformativeText(request.reason);
    alert.addButtonWithTitle("Open System Settings");
    alert.addButtonWithTitle("Later");

    if alert.runModal() == NSAlertFirstButtonReturn {
        // Open the relevant System Settings pane
        NSWorkspace::shared().open(request.settings_url());
    }
}
```

**Required permissions:**
| Permission | Features | Why |
|------------|----------|-----|
| Accessibility | window_title, active app, workspace | Read window/app info |

**Optional permissions:**
| Permission | Features | Why |
|------------|----------|-----|
| Screen Recording | Some window capture features | Only if specific features used |

## System Menu Bar

To fully replace the menu bar, auto-hide the system one:

```bash
# Hide menu bar (requires restart or re-login)
defaults write NSGlobalDomain _HIHideMenuBar -bool true

# Show menu bar again
defaults write NSGlobalDomain _HIHideMenuBar -bool false
```

Or in System Settings → Control Center → "Automatically hide and show the menu bar" → "Always"

RustyBar should prompt or document this during setup.

## Milestones (Incremental Releases)

### v0.1 — Proof of Concept
Get a visible bar on screen with basic rendering.

- [ ] Window creation with correct level (above other windows)
- [ ] Menu bar height detection (`"auto"`)
- [ ] Notch detection (split into two windows on notch displays)
- [ ] Basic Core Graphics rendering
- [ ] Single static module (clock) to prove rendering works
- [ ] TOML config loading (bar height, background color)

**Ship when**: Bar appears, shows a clock, respects notch.

---

### v0.2 — Layout Engine
Four-zone flex layout system.

- [ ] Four-zone layout (left.left, left.right, right.left, right.right)
- [ ] Flex vs fixed module widths
- [ ] Fake notch rendering (curves, exclusion zone)
- [ ] Multiple static modules
- [ ] Text rendering with Core Text (single style)

**Ship when**: Modules can be placed in all four zones, fake notch works.

---

### v0.3 — Module Rendering
Full visual styling for modules.

- [ ] Module layouts: inline, stacked (header/value), icon-only, badge
- [ ] Nerd Font icon rendering
- [ ] Background, border, corner_radius styling
- [ ] Multiple text sizes (header vs value)
- [ ] Built-in modules: clock, battery, cpu, memory
- [ ] Separators (line, dot, icon, space)
- [ ] Module grouping (shared background)
- [ ] Horizontal margin per module

**Ship when**: Modules look like the design (stacked labels, icons, badges, groups).

---

### v0.4 — Interaction
Hover states and click handling.

- [ ] NSTrackingArea per module (hover detection)
- [ ] Hover state rendering (color changes)
- [ ] Transitions (150ms default)
- [ ] Pointer cursor on clickable modules
- [ ] Click handlers (on_click → shell command)

**Ship when**: Hovering changes appearance, clicking runs commands.

---

### v0.5 — Dynamic Content
Live-updating modules and smooth layout changes.

- [ ] System stat polling (timers)
- [ ] Built-in modules: volume, wifi, disk, workspace, window_title
- [ ] Dynamic width transitions (content changes animate)
- [ ] Loading states for async modules
- [ ] Hot-reload configuration
- [ ] Error boundaries per module (failures isolated)
- [ ] Debug mode logging (`--debug` flag, log to file)

**Ship when**: Stats update live, layout animates, errors don't crash bar.

---

### v0.6 — Events & Integration
System event hooks and Aerospace integration.

- [ ] Space change notifications
- [ ] Window focus tracking
- [ ] App launch/quit events
- [ ] Display connect/disconnect handling
- [ ] Yabai workspace integration
- [ ] Menu bar auto-hide sync (fade in/out with system menu bar)

**Ship when**: Workspace module shows current space, window title updates on focus, bar syncs with menu bar visibility.

---

### v0.7 — Scripting
Custom modules via Rhai.

- [ ] Rhai runtime integration
- [ ] Script-based modules
- [ ] System APIs exposed to scripts (shell, http, system stats)
- [ ] Hot-reload scripts

**Ship when**: Users can write custom modules in Rhai.

---

### v1.0 — Release
Polish and distribution.

- [ ] IPC socket for external control (`rustybar set`, `rustybar reload`)
- [ ] Right-click context menu (reload, edit config, toggle debug, quit)
- [ ] Multi-monitor config overrides
- [ ] Vibrancy/blur effects (optional)
- [ ] Documentation
- [ ] Homebrew formula

**Ship when**: Stable enough for daily use, easy to install.

---

### v2.0 — Popups & Drawers (Future)

**Two popup styles:**

- [ ] **Drawer mode** — Bar extends downward like a drawer (preferred)
- [ ] **Floating mode** — Traditional popup below module
- [ ] Auto-dismiss on click outside
- [ ] Only one popup/drawer open at a time
- [ ] Content swap without transition when clicking another drawer item
- [ ] Scrollable content
- [ ] Calendar widget
- [ ] System monitor graphs
- [ ] Audio/WiFi pickers
- [ ] Menu bar item proxying

## References

- [SketchyBar source](https://github.com/FelixKratz/SketchyBar) — C implementation, good reference for macOS APIs
- [objc2 documentation](https://docs.rs/objc2)
- [Apple's NSWindow docs](https://developer.apple.com/documentation/appkit/nswindow)
- [Aerospace](https://github.com/koekeishiya/Aerospace) — Tiling WM to integrate with

## Decisions Made

- **Layout**: Always split at center, even on non-notch displays ✓
- **Fake notch**: Optional visual element for consistency across displays ✓
- **Scripting**: Rhai (Rust-native, no C deps, clean integration) ✓
- **v1 scope**: No popups, click-only (run commands) ✓
- **Module layouts**: Support stacked (header/value), inline, icon-only, badge ✓
- **Badge styling**: Any content, `corner_radius` (0 = square), `background` and/or `border` with configurable `border_width` ✓
- **Cursor**: Pointing hand on hover for any clickable module (automatic) ✓
- **Hover states**: All style properties (foreground, background, border, icon_color, header_color) can have hover variants ✓
- **Transitions**: 150ms default for hover states, customizable per-module or globally ✓
- **Loading states**: Spinner/pulse/dots animation for modules fetching external data ✓
- **Flex layout**: Modules can be fixed or flex width, shrink/grow like CSS flexbox ✓
- **Dynamic content**: Width changes animate smoothly (e.g., "5%" → "100%") ✓
- **Fake notch**: Draws curved notch shape (top curves outward, bottom curves inward), creates exclusion zone, configurable width/color/corner_radius ✓
- **Bar height**: `"auto"` to match system menu bar exactly, or specify pixels ✓
- **Release strategy**: Ship incrementally (v0.1 → v0.2 → ... → v1.0) ✓
- **Menu bar sync**: Frame-locked tracking via CVDisplayLink — follows actual menu bar position, perfectly synced without guessing easing curves ✓
- **Separators**: Line (pipe), dot, custom icon, or space ✓
- **Module grouping**: Multiple modules can share a background ✓
- **Error boundaries**: Each module isolated, failures don't crash bar ✓
- **Logging**: Debug mode flag, logs to file, LLM-friendly format, always on during dev ✓
- **Right-click menu**: Reload config, edit config, toggle debug, quit ✓
- **Module spacing**: Horizontal margin_left/margin_right per module (vertical alignment consistent) ✓
- **Conditional visibility**: Modules can return `visible: false` to hide dynamically (battery when charging, volume when normal, etc.) ✓
- **Module owns all states**: Each module defines its own default, hover, active, loading, and error appearances ✓
- **Drawer mode (v2)**: Bar extends downward as integrated drawer; app-level config for transition/border, module-level config for height only; content/height swaps smoothly ✓
- **Config location**: `~/.config/rustybar/config.toml`, scripts in `~/.config/rustybar/scripts/` ✓
- **Launch on startup**: Config-based LaunchAgent install (`launch_on_login = true`) ✓
- **Font fallback**: Icons show fallback text like `[bat]` if Nerd Font not installed ✓
- **Tiling WM**: Aerospace integration (not yabai) ✓
- **Permissions**: Prompt and guide user to System Settings when needed ✓
- **Fake notch**: Only applies to external monitors (real notch displays use actual notch) ✓
- **Module IDs**: Optional explicit `id`, auto-generated as `{type}-{index}` if not specified ✓
- **IPC**: Unix socket at `/tmp/rustybar.sock`, JSON protocol ✓
- **First run**: Generate default config.toml with starter setup ✓
- **Config errors**: Show red error banner in bar, clickable to open config ✓
- **Animation easing**: Ease-in-out for all transitions ✓
- **Colors**: Hex with transparency (`#RRGGBB` or `#RRGGBBAA`) ✓
- **Icons**: Nerd Fonts ✓

## Open Questions

### Core
- Use private CGS APIs (like SketchyBar) or stick to public APIs only?
- How to handle notch detection on older macOS versions without safe area APIs?
- Minimum supported macOS version? (12+ for safe area APIs, or support older?)

### Rendering
- Should bar support transparency/blur, or solid background only for v1?
- Animation framework: Core Animation, or manual frame interpolation?

### Integration (v2)
- Calendar popup: System Calendar.app events or standalone?
- Menu bar proxying: Worth the complexity?
