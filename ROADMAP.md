# Roadmap

## Planned Features

### Logging Module

A full-drawer popup module for viewing runtime logs and errors.

**Behavior:**
- Defaults to closed (not open on startup)
- Shows standard log icon in bar when no errors
- Shows red error icon when runtime errors exist (clickable to open)
- Click icon to open full popup drawer

**Popup Content:**
- Lists logs and errors with timestamps
- Error entries highlighted/distinguished from info logs
- Scrollable log history
- Option to clear logs

**Implementation Notes:**
- Capture internal Sinew logs (config errors, module failures, IPC issues)
- Store in ring buffer to limit memory usage
- Error state tracked globally to update icon appearance
