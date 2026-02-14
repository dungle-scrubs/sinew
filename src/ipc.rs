//! IPC command types, global command bus, and Unix socket listener.
//!
//! Commands are parsed from the socket, pushed onto an async channel,
//! and drained by the GPUI bar view on each render frame.

use async_channel::{Receiver, Sender};
use std::sync::{Mutex, OnceLock};

use crate::gpui_app::modules::external::get_external_state;
use crate::gpui_app::request_immediate_refresh;

/// An IPC command destined for the GPUI main thread.
#[derive(Debug, Clone)]
pub enum IpcCommand {
    /// Set one or more properties on a module.
    Set {
        module_id: String,
        properties: Vec<(String, String)>,
    },
    /// Trigger a module event (e.g. "update" or "popup").
    Trigger { module_id: String, event: String },
}

/// Async channel pair for IPC → GPUI communication.
struct IpcCommandBus {
    tx: Sender<IpcCommand>,
    rx: Receiver<IpcCommand>,
}

static IPC_COMMAND_BUS: OnceLock<IpcCommandBus> = OnceLock::new();

/// Returns (or initialises) the global IPC command bus.
fn command_bus() -> &'static IpcCommandBus {
    IPC_COMMAND_BUS.get_or_init(|| {
        let (tx, rx) = async_channel::unbounded();
        IpcCommandBus { tx, rx }
    })
}

/// Returns a receiver for the bar's drain loop.
pub fn subscribe_ipc_commands() -> Receiver<IpcCommand> {
    command_bus().rx.clone()
}

/// Pushes a command onto the bus and wakes the render loop.
fn push_ipc_command(cmd: IpcCommand) {
    let _ = command_bus().tx.try_send(cmd);
    request_immediate_refresh();
}

// ---------------------------------------------------------------------------
// Module ID/type registry (for `list` command)
// ---------------------------------------------------------------------------

static MODULE_ID_TYPE_MAP: OnceLock<Mutex<Vec<(String, String)>>> = OnceLock::new();

fn id_type_map() -> &'static Mutex<Vec<(String, String)>> {
    MODULE_ID_TYPE_MAP.get_or_init(|| Mutex::new(Vec::new()))
}

/// Records a module's id and type when it is created.
pub fn register_module_id(id: &str, module_type: &str) {
    if let Ok(mut map) = id_type_map().lock() {
        map.push((id.to_string(), module_type.to_string()));
    }
}

/// Clears the registry (called on config reload).
pub fn clear_module_ids() {
    if let Ok(mut map) = id_type_map().lock() {
        map.clear();
    }
}

/// Returns all registered module (id, type) pairs.
fn all_module_ids() -> Vec<(String, String)> {
    id_type_map().lock().map(|v| v.clone()).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Command parsing
// ---------------------------------------------------------------------------

/// Parses and dispatches a single IPC command string, returning a response.
pub fn handle_ipc_command(command: &str) -> String {
    let trimmed = command.trim();
    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
    let verb = parts.first().copied().unwrap_or("");

    match verb {
        "reload" | "redraw" => {
            request_immediate_refresh();
            "OK: refresh requested".to_string()
        }
        "status" => {
            let status = serde_json::json!({
                "version": crate::VERSION,
                "running": true,
            });
            status.to_string()
        }
        "set" => handle_set(parts.get(1).copied().unwrap_or("")),
        "get" => handle_get(parts.get(1).copied().unwrap_or("")),
        "list" => handle_list(),
        "trigger" => handle_trigger(parts.get(1).copied().unwrap_or("")),
        other => format!("ERR: unknown command '{}'", other),
    }
}

/// `set <module_id> key=value [key=value ...]`
fn handle_set(args: &str) -> String {
    let mut tokens = args.split_whitespace();
    let Some(module_id) = tokens.next() else {
        return "ERR: set requires <module_id> key=value".to_string();
    };

    let mut properties = Vec::new();
    for token in tokens {
        if let Some((key, value)) = parse_kv(token) {
            properties.push((key, value));
        } else {
            return format!("ERR: invalid key=value pair '{}'", token);
        }
    }

    if properties.is_empty() {
        return "ERR: set requires at least one key=value pair".to_string();
    }

    push_ipc_command(IpcCommand::Set {
        module_id: module_id.to_string(),
        properties,
    });
    "OK".to_string()
}

/// Parses `key=value` or `key="quoted value"`.
fn parse_kv(token: &str) -> Option<(String, String)> {
    let eq = token.find('=')?;
    let key = token[..eq].to_string();
    let raw_value = &token[eq + 1..];
    let value = raw_value
        .trim_start_matches('"')
        .trim_end_matches('"')
        .to_string();
    Some((key, value))
}

/// `get <module_id> [property]` — reads ExternalState directly (no GPUI round-trip).
fn handle_get(args: &str) -> String {
    let mut tokens = args.split_whitespace();
    let Some(module_id) = tokens.next() else {
        return "ERR: get requires <module_id>".to_string();
    };
    let property = tokens.next();

    let Some(state) = get_external_state(module_id) else {
        return format!("ERR: module '{}' not found or not external", module_id);
    };

    let Ok(guard) = state.lock() else {
        return "ERR: state lock contention".to_string();
    };

    if let Some(prop) = property {
        match prop {
            "label" => guard.label.clone(),
            "icon" => guard.icon.clone().unwrap_or_default(),
            "color" => format_opt_color(guard.color),
            "background" => format_opt_color(guard.background),
            "drawing" => if guard.drawing { "on" } else { "off" }.to_string(),
            other => format!("ERR: unknown property '{}'", other),
        }
    } else {
        // Return all properties as key=value lines
        let mut out = Vec::new();
        out.push(format!("label={}", guard.label));
        if let Some(ref icon) = guard.icon {
            out.push(format!("icon={}", icon));
        }
        out.push(format!(
            "drawing={}",
            if guard.drawing { "on" } else { "off" }
        ));
        if let Some(c) = guard.color {
            out.push(format!("color={}", rgba_to_hex(c)));
        }
        if let Some(c) = guard.background {
            out.push(format!("background={}", rgba_to_hex(c)));
        }
        out.join("\n")
    }
}

/// `list` — returns JSON array of all module ids and types.
fn handle_list() -> String {
    let entries: Vec<serde_json::Value> = all_module_ids()
        .into_iter()
        .map(|(id, t)| serde_json::json!({"id": id, "type": t}))
        .collect();
    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
}

/// `trigger <module_id> update|popup`
fn handle_trigger(args: &str) -> String {
    let mut tokens = args.split_whitespace();
    let Some(module_id) = tokens.next() else {
        return "ERR: trigger requires <module_id> <event>".to_string();
    };
    let Some(event) = tokens.next() else {
        return "ERR: trigger requires <event> (update|popup)".to_string();
    };

    push_ipc_command(IpcCommand::Trigger {
        module_id: module_id.to_string(),
        event: event.to_string(),
    });
    "OK".to_string()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Converts an optional Rgba to a hex display string.
fn format_opt_color(c: Option<gpui::Rgba>) -> String {
    match c {
        Some(c) => rgba_to_hex(c),
        None => String::new(),
    }
}

/// Converts an Rgba to `#RRGGBB` hex string.
fn rgba_to_hex(c: gpui::Rgba) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (c.r * 255.0) as u8,
        (c.g * 255.0) as u8,
        (c.b * 255.0) as u8,
    )
}

// ---------------------------------------------------------------------------
// Unix socket listener (extracted from main.rs)
// ---------------------------------------------------------------------------

/// Starts the IPC listener on a Unix socket, spawning a background thread.
pub fn start_ipc_listener(socket_path: &std::path::Path) -> std::io::Result<()> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream};

    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = match UnixListener::bind(socket_path) {
        Ok(listener) => listener,
        Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
            if UnixStream::connect(socket_path).is_ok() {
                eprintln!("Sinew is already running.");
                std::process::exit(0);
            }
            let _ = std::fs::remove_file(socket_path);
            UnixListener::bind(socket_path)?
        }
        Err(err) => return Err(err),
    };

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            let response = handle_ipc_command(&line);
            if let Ok(mut stream) = reader.into_inner().try_clone() {
                let _ = writeln!(stream, "{}", response);
            }
        }
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- parse_kv -----------------------------------------------------------

    #[test]
    fn parse_kv_simple() {
        let (k, v) = parse_kv("label=hello").unwrap();
        assert_eq!(k, "label");
        assert_eq!(v, "hello");
    }

    #[test]
    fn parse_kv_quoted_value() {
        let (k, v) = parse_kv("label=\"hello world\"").unwrap();
        assert_eq!(k, "label");
        assert_eq!(v, "hello world");
    }

    #[test]
    fn parse_kv_empty_value() {
        let (k, v) = parse_kv("icon=").unwrap();
        assert_eq!(k, "icon");
        assert_eq!(v, "");
    }

    #[test]
    fn parse_kv_value_with_equals() {
        let (k, v) = parse_kv("label=a=b").unwrap();
        assert_eq!(k, "label");
        assert_eq!(v, "a=b");
    }

    #[test]
    fn parse_kv_no_equals_returns_none() {
        assert!(parse_kv("nope").is_none());
    }

    #[test]
    fn parse_kv_emoji_value() {
        let (k, v) = parse_kv("icon=🔥").unwrap();
        assert_eq!(k, "icon");
        assert_eq!(v, "🔥");
    }

    // -- rgba_to_hex --------------------------------------------------------

    #[test]
    fn rgba_to_hex_black() {
        let c = gpui::Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
        assert_eq!(rgba_to_hex(c), "#000000");
    }

    #[test]
    fn rgba_to_hex_white() {
        let c = gpui::Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        assert_eq!(rgba_to_hex(c), "#ffffff");
    }

    #[test]
    fn rgba_to_hex_red() {
        let c = gpui::Rgba { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
        assert_eq!(rgba_to_hex(c), "#ff0000");
    }

    #[test]
    fn rgba_to_hex_ignores_alpha() {
        let c = gpui::Rgba { r: 0.5, g: 0.5, b: 0.5, a: 0.0 };
        assert_eq!(rgba_to_hex(c), "#7f7f7f");
    }

    // -- format_opt_color ---------------------------------------------------

    #[test]
    fn format_opt_color_none() {
        assert_eq!(format_opt_color(None), "");
    }

    #[test]
    fn format_opt_color_some() {
        let c = gpui::Rgba { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
        assert_eq!(format_opt_color(Some(c)), "#ff0000");
    }

    // -- handle_set error paths ---------------------------------------------

    #[test]
    fn handle_set_missing_module_id() {
        let resp = handle_set("");
        assert!(resp.starts_with("ERR:"));
    }

    #[test]
    fn handle_set_no_properties() {
        let resp = handle_set("mymod");
        assert!(resp.starts_with("ERR:"));
        assert!(resp.contains("key=value"));
    }

    #[test]
    fn handle_set_invalid_kv() {
        let resp = handle_set("mymod nope");
        assert!(resp.starts_with("ERR:"));
        assert!(resp.contains("nope"));
    }

    // -- handle_get error paths ---------------------------------------------

    #[test]
    fn handle_get_missing_module_id() {
        let resp = handle_get("");
        assert!(resp.starts_with("ERR:"));
    }

    #[test]
    fn handle_get_unknown_module() {
        let resp = handle_get("nonexistent_module_xyz");
        assert!(resp.starts_with("ERR:"));
        assert!(resp.contains("not found"));
    }

    // -- handle_trigger error paths -----------------------------------------

    #[test]
    fn handle_trigger_missing_module_id() {
        let resp = handle_trigger("");
        assert!(resp.starts_with("ERR:"));
    }

    #[test]
    fn handle_trigger_missing_event() {
        let resp = handle_trigger("mymod");
        assert!(resp.starts_with("ERR:"));
        assert!(resp.contains("event"));
    }

    // -- handle_list --------------------------------------------------------

    #[test]
    fn handle_list_returns_json_array() {
        let resp = handle_list();
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert!(parsed.is_array());
    }

    // -- module ID registry -------------------------------------------------

    #[test]
    fn register_and_list_module_ids() {
        register_module_id("test-ipc-mod", "external");
        let ids = all_module_ids();
        assert!(ids.iter().any(|(id, t)| id == "test-ipc-mod" && t == "external"));
    }
}
