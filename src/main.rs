// Allow complex types in internal code
#![allow(clippy::type_complexity)]
// Allow functions with many arguments for now
#![allow(clippy::too_many_arguments)]

mod config;
mod gpui_app;
mod window;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn socket_path() -> std::path::PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(runtime_dir).join("rustybar.sock")
}

/// Removes the Unix socket file on process exit.
fn install_socket_cleanup() {
    let socket = socket_path();
    // Register cleanup for SIGINT/SIGTERM
    let socket_clone = socket.clone();
    if let Err(e) = ctrlc::set_handler(move || {
        let _ = std::fs::remove_file(&socket_clone);
        std::process::exit(0);
    }) {
        log::warn!("Failed to install signal handler: {}", e);
    }
}

/// Handles an IPC command and returns the response string.
fn handle_ipc_command(command: &str) -> String {
    let parts: Vec<&str> = command.trim().splitn(2, ' ').collect();
    match parts.first().copied().unwrap_or("") {
        "reload" | "redraw" => {
            gpui_app::request_immediate_refresh();
            "OK: refresh requested".to_string()
        }
        "status" => {
            let status = serde_json::json!({
                "version": VERSION,
                "running": true,
            });
            status.to_string()
        }
        other => {
            format!("ERR: unknown command '{}'", other)
        }
    }
}

fn start_ipc_listener() -> std::io::Result<()> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream};

    let socket = socket_path();
    if let Some(parent) = socket.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = match UnixListener::bind(&socket) {
        Ok(listener) => listener,
        Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
            if UnixStream::connect(&socket).is_ok() {
                eprintln!("RustyBar is already running.");
                std::process::exit(0);
            }
            let _ = std::fs::remove_file(&socket);
            UnixListener::bind(&socket)?
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

fn print_help() {
    println!(
        "rustybar {}
A macOS menu bar replacement with notch-aware layouts

USAGE:
    rustybar [OPTIONS]

OPTIONS:
    -h, --help       Print this help message
    -v, --version    Print version information

ENVIRONMENT:
    RUST_LOG         Set log level (error, warn, info, debug, trace)

CONFIG:
    ~/.config/rustybar/config.toml

EXAMPLES:
    rustybar                    Run with default config
    RUST_LOG=debug rustybar     Run with debug logging

For more information, see: https://github.com/dungle-scrubs/rustybar",
        VERSION
    );
}

fn main() {
    // Handle CLI arguments
    let args: Vec<String> = std::env::args().skip(1).collect();

    if !args.is_empty() {
        // Only the first argument is processed (flags don't combine)
        match args[0].as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-v" | "--version" => {
                println!("rustybar {}", VERSION);
                return;
            }
            _ => {
                eprintln!("Unknown argument: {}", args[0]);
                eprintln!("Try 'rustybar --help' for more information.");
                std::process::exit(1);
            }
        }
    }

    // Initialize logging (flush each line for interactive debugging).
    let mut logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    logger
        .format(|buf, record| {
            use std::io::Write;
            writeln!(
                buf,
                "[{} {:>5} {}] {}",
                chrono::Utc::now().to_rfc3339(),
                record.level(),
                record.target(),
                record.args()
            )?;
            buf.flush()
        })
        .init();

    log::info!("Starting RustyBar v{}", VERSION);

    if let Err(err) = start_ipc_listener() {
        log::warn!("Failed to start IPC listener: {}", err);
    }
    install_socket_cleanup();

    // Run the GPUI-based application
    gpui_app::run();
}
