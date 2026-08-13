//! Starting and stopping the local language-model service.
//!
//! Refinement through Ollama is Murmel's default, which only works if Ollama is
//! actually running. Leaving that to the user means sending them to a terminal
//! or hoping an autostart entry fired — and when it did not, the failure is
//! silent: the dictation simply arrives unrefined.
//!
//! Murmel therefore runs `ollama serve` itself. That is a background process
//! with no window of its own, which is only acceptable because Murmel makes it
//! visible and controllable: the settings screen shows whether it is running,
//! who started it, and offers to start or stop it. An invisible process would
//! not be.
//!
//! Two limits remain:
//!
//! - **Only what Murmel started gets stopped.** A service the user or another
//!   tool launched is left alone — something else may be talking to it.
//! - **Nothing is installed.** Murmel launches a program the user chose to
//!   have; it does not download one.

use log::{debug, info, warn};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long to wait for the service to answer after launching it. Ollama has to
/// bind its port and load its runtime; a couple of seconds is normal on a cold
/// start, and failing earlier would report a problem that isn't one.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(400);

/// The child process, while Murmel is the one running it.
///
/// Kept so the service can be stopped again — and so the UI can say "started by
/// Murmel" rather than merely "running", which is the difference between a
/// background process and an invisible one.
static OWNED: Mutex<Option<Child>> = Mutex::new(None);

/// Locate the Ollama executable.
///
/// Deliberately **not** the tray application (`ollama app.exe`): launching it
/// brings up its window without reliably serving the API, so Murmel would
/// report success while nothing listens. `ollama serve` is the documented way
/// to run the server, and it is the one that works.
fn find_ollama() -> Option<PathBuf> {
    #[cfg(windows)]
    let candidates = {
        let mut candidates = Vec::new();
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(local)
                    .join("Programs")
                    .join("Ollama")
                    .join("ollama.exe"),
            );
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

/// Process id of the service Murmel started, if it is still ours.
pub fn owned_pid() -> Option<u32> {
    let mut guard = OWNED.lock().ok()?;

    // `try_wait` reaps the child if it exited on its own, so a crashed service
    // does not keep being reported as running.
    if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(Some(status)) => {
                debug!("Local LLM service exited on its own ({status})");
                *guard = None;
                return None;
            }
            Ok(None) => return Some(child.id()),
            Err(err) => {
                warn!("Could not query local LLM service: {err}");
                return None;
            }
        }
    }

    None
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

    info!("Starting local LLM service: {} serve", path.display());
    let mut command = Command::new(&path);
    command.arg("serve");

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

    let child = command
        .spawn()
        .map_err(|err| format!("Could not start Ollama: {err}"))?;

    debug!("Local LLM service started as pid {}", child.id());
    if let Ok(mut guard) = OWNED.lock() {
        *guard = Some(child);
    }

    Ok(())
}

/// Stop the service — but only if Murmel is the one that started it.
pub fn stop_ollama() -> Result<(), String> {
    let mut guard = OWNED
        .lock()
        .map_err(|_| "Could not access the service handle.".to_string())?;

    let Some(child) = guard.as_mut() else {
        return Err(
            "This service was not started by Murmel, so Murmel does not stop it. \
             Something else may be using it."
                .to_string(),
        );
    };

    child
        .kill()
        .map_err(|err| format!("Could not stop Ollama: {err}"))?;
    // Reap immediately; otherwise the process lingers as a zombie on Unix.
    let _ = child.wait();

    info!("Local LLM service stopped");
    *guard = None;
    Ok(())
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

    /// Nothing to stop means a clear message, not a silent success that leaves
    /// the user wondering whether anything happened.
    #[test]
    fn stopping_a_service_murmel_did_not_start_is_refused() {
        assert!(OWNED.lock().unwrap().is_none());
        assert!(stop_ollama().is_err());
    }

    #[test]
    fn no_owned_pid_before_starting_anything() {
        assert_eq!(owned_pid(), None);
    }
}
