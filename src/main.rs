// Allow dead code for API methods/structs meant for future use
#![allow(dead_code)]
// Allow complex types in internal code
#![allow(clippy::type_complexity)]
// Allow functions with many arguments for now
#![allow(clippy::too_many_arguments)]

mod config;
mod gpui_app;
mod window;

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

    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting RustyBar v{}", VERSION);

    // Run the GPUI-based application
    gpui_app::run();
}
