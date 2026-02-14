// Allow complex types in internal code
#![allow(clippy::type_complexity)]
// Allow functions with many arguments for now
#![allow(clippy::too_many_arguments)]

mod config;
mod gpui_app;
mod ipc;
mod window;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn socket_path() -> std::path::PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(runtime_dir).join("sinew.sock")
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

fn start_ipc_listener() -> std::io::Result<()> {
    ipc::start_ipc_listener(&socket_path())
}

fn print_help() {
    println!(
        "sinew {}
A macOS menu bar replacement with notch-aware layouts

USAGE:
    sinew [OPTIONS]

OPTIONS:
    -h, --help       Print this help message
    -v, --version    Print version information

ENVIRONMENT:
    RUST_LOG         Set log level (error, warn, info, debug, trace)

CONFIG:
    ~/.config/sinew/config.toml

EXAMPLES:
    sinew                    Run with default config
    RUST_LOG=debug sinew     Run with debug logging

For more information, see: https://github.com/dungle-scrubs/sinew",
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
                println!("sinew {}", VERSION);
                return;
            }
            _ => {
                eprintln!("Unknown argument: {}", args[0]);
                eprintln!("Try 'sinew --help' for more information.");
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

    log::info!("Starting Sinew v{}", VERSION);

    if let Err(err) = start_ipc_listener() {
        log::warn!("Failed to start IPC listener: {}", err);
    }
    install_socket_cleanup();

    // Run the GPUI-based application
    gpui_app::run();
}
