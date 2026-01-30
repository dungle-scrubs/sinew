// Allow dead code during development - remove before release
#![allow(dead_code)]
#![allow(unused_variables)]

mod app;
mod config;
mod ipc;
mod modules;
mod render;
mod view;
mod window;

use objc2::MainThreadMarker;

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting RustyBar v0.1");

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
