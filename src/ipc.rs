use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

/// IPC server for external control of RustyBar
pub struct IpcServer {
    socket_path: PathBuf,
    running: Arc<AtomicBool>,
}

/// Commands that can be sent via IPC
#[derive(Debug)]
pub enum IpcCommand {
    /// Trigger a redraw
    Redraw,
    /// Reload configuration
    Reload,
    /// Get bar status (returns JSON)
    Status,
    /// Update a module's value (module_id, value)
    SetValue(String, String),
    /// Show/hide the bar
    Toggle,
    /// Unknown command
    Unknown(String),
}

impl IpcServer {
    pub fn new() -> Self {
        let socket_path = Self::socket_path();
        Self {
            socket_path,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn socket_path() -> PathBuf {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(runtime_dir).join("rustybar.sock")
    }

    pub fn start<F>(&self, handler: F) -> std::io::Result<()>
    where
        F: Fn(IpcCommand) -> String + Send + 'static,
    {
        // Remove existing socket if present
        let _ = std::fs::remove_file(&self.socket_path);

        let listener = UnixListener::bind(&self.socket_path)?;
        listener.set_nonblocking(true)?;

        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let socket_path = self.socket_path.clone();

        thread::spawn(move || {
            log::info!("IPC server listening on {:?}", socket_path);

            while running.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let handler_ref = &handler;
                        Self::handle_client(stream, handler_ref);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No connection ready, sleep briefly
                        thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(e) => {
                        log::error!("IPC accept error: {}", e);
                    }
                }
            }

            let _ = std::fs::remove_file(&socket_path);
            log::info!("IPC server stopped");
        });

        Ok(())
    }

    fn handle_client<F>(mut stream: UnixStream, handler: &F)
    where
        F: Fn(IpcCommand) -> String,
    {
        let reader = BufReader::new(stream.try_clone().unwrap());

        for line in reader.lines() {
            match line {
                Ok(cmd_str) => {
                    let cmd = Self::parse_command(&cmd_str);
                    let response = handler(cmd);
                    if let Err(e) = writeln!(stream, "{}", response) {
                        log::error!("IPC write error: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    log::error!("IPC read error: {}", e);
                    break;
                }
            }
        }
    }

    fn parse_command(input: &str) -> IpcCommand {
        let parts: Vec<&str> = input.trim().splitn(3, ' ').collect();
        match parts.get(0).map(|s| s.to_lowercase()).as_deref() {
            Some("redraw") => IpcCommand::Redraw,
            Some("reload") => IpcCommand::Reload,
            Some("status") => IpcCommand::Status,
            Some("toggle") => IpcCommand::Toggle,
            Some("set") if parts.len() >= 3 => {
                IpcCommand::SetValue(parts[1].to_string(), parts[2].to_string())
            }
            _ => IpcCommand::Unknown(input.to_string()),
        }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Default for IpcServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Send a command to a running RustyBar instance
pub fn send_command(command: &str) -> std::io::Result<String> {
    let socket_path = IpcServer::socket_path();
    let mut stream = UnixStream::connect(&socket_path)?;

    writeln!(stream, "{}", command)?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;

    Ok(response.trim().to_string())
}
