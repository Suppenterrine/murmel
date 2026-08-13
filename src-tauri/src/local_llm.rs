//! Starting the local language-model service.
//!
//! Refinement through Ollama is Murmel's default, which only works if Ollama is
//! actually running. Leaving that to the user means sending them to a terminal
//! or hoping an autostart entry fired — and when it did not, the failure is
//! silent: the dictation simply arrives unrefined.
//!
//! So Murmel starts the service itself. Two deliberate limits:
//!
//! - **It never stops it again.** The process belongs to the user, not to
//!   Murmel; other tools may be talking to the same instance.
//! - **It only starts what is already installed.** Nothing is downloaded and
//!   nothing is installed — Murmel launches a program the user chose to have.

use log::{debug, info, warn};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

/// How long to wait for the service to answer after launching it. Ollama has to
/// bind its port and load its runtime; a couple of seconds is normal on a cold
/// start, and failing earlier would report a problem that isn't one.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(400);

/// Locate the Ollama executable.
///
/// The PATH is checked first — a user who installed Ollama elsewhere has it
/// there — with the platform's default install location as a fallback, because
/// on Windows the installer does not always reach an already-running shell's
/// environment.
fn find_ollama() -> Option<PathBuf> {
    #[cfg(windows)]
    let candidates = {
        let mut candidates = Vec::new();
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let base = PathBuf::from(local).join("Programs").join("Ollama");
            // The tray application is preferred: it starts the server the same
            // way the user would, complete with its tray icon, instead of
            // leaving an invisible orphan process behind.
            candidates.push(base.join("ollama app.exe"));
            candidates.push(base.join("ollama.exe"));
        }
        candidates.push(PathBuf::from("ollama.exe"));
        candidates
    };

    #[cfg(not(windows))]
    let candidates = vec![
        PathBuf::from("/usr/local/bin/ollama"),
        PathBuf::from("/usr/bin/ollama"),
        PathBuf::from("/opt/homebrew/bin/ollama"),
        PathBuf::from("ollama"),
    ];

    candidates.into_iter().find(|path| {
        // A bare file name has to be resolved through PATH, which `exists()`
        // cannot do — try running it instead.
        if path.components().count() == 1 {
            Command::new(path)
                .arg("--version")
                .output()
                .is_ok_and(|out| out.status.success() || !out.stderr.is_empty())
        } else {
            path.exists()
        }
    })
}

/// True when the executable is Ollama's tray application, which starts the
/// server on its own and takes no `serve` argument.
fn is_tray_app(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("ollama app.exe"))
}

/// Launch the service. Returns once it has been spawned — not once it answers.
pub fn spawn_ollama() -> Result<(), String> {
    let Some(path) = find_ollama() else {
        return Err(
            "Ollama was not found on this machine. Install it from ollama.com, \
             or point the base URL at a service running elsewhere."
                .to_string(),
        );
    };

    info!("Starting local LLM service: {}", path.display());
    let mut command = Command::new(&path);
    if !is_tray_app(&path) {
        command.arg("serve");
    }

    // Without this the server would inherit Murmel's stdio and, on Windows,
    // flash up a console window.
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
        .spawn()
        .map(|child| {
            // The handle is dropped on purpose: the service outlives Murmel and
            // is not ours to wait on or reap.
            debug!("Local LLM service started as pid {}", child.id());
        })
        .map_err(|err| format!("Could not start Ollama: {err}"))
}

/// Start the service and wait until it answers, or give up.
///
/// `is_ready` is passed in rather than called from here so this module stays
/// free of any knowledge about providers and HTTP.
pub async fn start_and_wait<F, Fut>(is_ready: F) -> Result<(), String>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    if is_ready().await {
        debug!("Local LLM service already running");
        return Ok(());
    }

    spawn_ollama()?;

    let deadline = Instant::now() + STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        tokio::time::sleep(POLL_INTERVAL).await;
        if is_ready().await {
            info!("Local LLM service is up");
            return Ok(());
        }
    }

    warn!("Local LLM service did not answer within {STARTUP_TIMEOUT:?}");
    Err(
        "Ollama was started but is not responding yet. Give it a moment and check again."
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_app_is_recognised_by_name() {
        assert!(is_tray_app(std::path::Path::new(
            r"C:\Users\x\AppData\Local\Programs\Ollama\ollama app.exe"
        )));
        // Case differences must not decide whether `serve` is appended —
        // passing it to the tray app would make the launch fail.
        assert!(is_tray_app(std::path::Path::new("OLLAMA APP.EXE")));
        assert!(!is_tray_app(std::path::Path::new("ollama.exe")));
        assert!(!is_tray_app(std::path::Path::new("/usr/bin/ollama")));
    }
}
