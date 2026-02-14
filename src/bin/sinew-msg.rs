//! Command-line tool to send messages to a running Sinew instance

use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

fn socket_path() -> PathBuf {
    let runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(runtime_dir).join("sinew.sock")
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("Usage: sinew-msg <command> [args...]");
        eprintln!();
        eprintln!("Commands:");
        eprintln!("  redraw                          Trigger a bar redraw");
        eprintln!("  reload                          Reload configuration");
        eprintln!("  status                          Get bar status (JSON)");
        eprintln!("  set <id> key=value [...]        Set module properties");
        eprintln!("  get <id> [property]             Get module properties");
        eprintln!("  list                            List all modules (JSON)");
        eprintln!("  trigger <id> update|popup       Trigger module event");
        std::process::exit(1);
    }

    let command = args.join(" ");
    let socket = socket_path();

    match UnixStream::connect(&socket) {
        Ok(mut stream) => {
            if let Err(e) = writeln!(stream, "{}", command) {
                eprintln!("Failed to send command: {}", e);
                std::process::exit(1);
            }

            let mut reader = BufReader::new(stream);
            let mut response = String::new();
            if let Err(e) = reader.read_line(&mut response) {
                eprintln!("Failed to read response: {}", e);
                std::process::exit(1);
            }

            println!("{}", response.trim());
        }
        Err(e) => {
            eprintln!("Failed to connect to Sinew at {:?}: {}", socket, e);
            eprintln!("Is Sinew running?");
            std::process::exit(1);
        }
    }
}
