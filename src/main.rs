mod app;
mod components;
mod config;
mod ipc;
mod modules;
mod render;
mod view;
mod window;

use objc2::MainThreadMarker;

const VERSION: &str = env!("CARGO_PKG_VERSION");

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
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-v" | "--version" => {
                println!("rustybar {}", VERSION);
                return;
            }
            arg => {
                eprintln!("Unknown argument: {}", arg);
                eprintln!("Try 'rustybar --help' for more information.");
                std::process::exit(1);
            }
        }
    }

    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting RustyBar v{}", VERSION);

    // Start IPC server
    let ipc_server = ipc::IpcServer::new();
    if let Err(e) = ipc_server.start(|cmd| {
        match cmd {
            ipc::IpcCommand::Redraw => {
                view::bump_config_version();
                "ok".to_string()
            }
            ipc::IpcCommand::Reload => {
                view::bump_config_version();
                "ok".to_string()
            }
            ipc::IpcCommand::Status => r#"{"status":"running"}"#.to_string(),
            ipc::IpcCommand::Toggle => {
                // TODO: implement toggle
                "ok".to_string()
            }
            ipc::IpcCommand::SetValue(id, value) => {
                log::info!("Set {} = {}", id, value);
                "ok".to_string()
            }
            ipc::IpcCommand::Unknown(s) => {
                format!("error: unknown command: {}", s)
            }
        }
    }) {
        log::warn!("Failed to start IPC server: {}", e);
    }

    // Must run on main thread for AppKit
    let mtm = MainThreadMarker::new().expect("Must run on main thread");

    // Create and run application (config is loaded internally with hot reload)
    let app = app::App::new(mtm);
    app.run(mtm);
}
