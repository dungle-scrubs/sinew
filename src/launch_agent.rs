//! Manages the macOS launchd agent for "launch at login" functionality.
//!
//! Installs or removes `~/Library/LaunchAgents/com.sinew.bar.plist` based on
//! the `bar.launch_at_login` config option. The plist points to the currently
//! running binary so it works for both debug and release builds.

use std::path::PathBuf;

const PLIST_LABEL: &str = "com.sinew.bar";

/// Returns the path to the launch agent plist.
fn plist_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| {
        h.join("Library/LaunchAgents")
            .join(format!("{}.plist", PLIST_LABEL))
    })
}

/// Returns the path to the currently running sinew binary.
fn binary_path() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

/// Generates the plist XML content for the launch agent.
///
/// @param bin_path - Absolute path to the sinew binary
/// @returns Plist XML string
fn plist_contents(bin_path: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{PLIST_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{bin_path}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
    <key>StandardOutPath</key>
    <string>/tmp/sinew.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/sinew.stderr.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>RUST_LOG</key>
        <string>info</string>
    </dict>
</dict>
</plist>
"#
    )
}

/// Installs or removes the launch agent based on the desired state.
///
/// @param enabled - Whether launch-at-login should be active
pub fn sync(enabled: bool) {
    let Some(plist) = plist_path() else {
        log::warn!("Could not determine LaunchAgents directory");
        return;
    };

    let currently_installed = plist.exists();

    if enabled && !currently_installed {
        install(&plist);
    } else if !enabled && currently_installed {
        uninstall(&plist);
    } else if enabled && currently_installed {
        // Update binary path if it changed (e.g. switched from debug to release)
        update_if_needed(&plist);
    }
}

/// Installs the launch agent plist.
///
/// @param plist - Path to write the plist file
fn install(plist: &PathBuf) {
    let Some(bin) = binary_path() else {
        log::error!("Cannot install launch agent: unable to determine binary path");
        return;
    };

    // Ensure LaunchAgents directory exists
    if let Some(parent) = plist.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                log::error!("Failed to create LaunchAgents directory: {}", e);
                return;
            }
        }
    }

    let contents = plist_contents(&bin.to_string_lossy());
    match std::fs::write(plist, &contents) {
        Ok(()) => {
            log::info!("Installed launch agent at {:?} → {:?}", plist, bin);
            // Load the agent so it takes effect without requiring logout
            let _ = std::process::Command::new("launchctl")
                .args(["load", &plist.to_string_lossy()])
                .output();
        }
        Err(e) => log::error!("Failed to write launch agent plist: {}", e),
    }
}

/// Removes the launch agent plist.
///
/// @param plist - Path to the plist file to remove
fn uninstall(plist: &PathBuf) {
    // Unload before removing
    let _ = std::process::Command::new("launchctl")
        .args(["unload", &plist.to_string_lossy()])
        .output();

    match std::fs::remove_file(plist) {
        Ok(()) => log::info!("Removed launch agent at {:?}", plist),
        Err(e) => log::error!("Failed to remove launch agent plist: {}", e),
    }
}

/// Updates the plist if the binary path has changed.
///
/// @param plist - Path to the existing plist file
fn update_if_needed(plist: &PathBuf) {
    let Some(bin) = binary_path() else {
        return;
    };

    let expected = plist_contents(&bin.to_string_lossy());
    let current = std::fs::read_to_string(plist).unwrap_or_default();

    if current != expected {
        log::info!("Updating launch agent binary path → {:?}", bin);
        // Unload old, write new, reload
        let _ = std::process::Command::new("launchctl")
            .args(["unload", &plist.to_string_lossy()])
            .output();
        if let Err(e) = std::fs::write(plist, &expected) {
            log::error!("Failed to update launch agent plist: {}", e);
            return;
        }
        let _ = std::process::Command::new("launchctl")
            .args(["load", &plist.to_string_lossy()])
            .output();
    }
}
