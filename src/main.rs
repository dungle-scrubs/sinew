mod app;
mod config;
mod modules;
mod render;
mod view;
mod window;

use objc2::MainThreadMarker;

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting RustyBar v0.1");

    // Must run on main thread for AppKit
    let mtm = MainThreadMarker::new().expect("Must run on main thread");

    // Create and run application (config is loaded internally with hot reload)
    let app = app::App::new(mtm);
    app.run(mtm);
}
