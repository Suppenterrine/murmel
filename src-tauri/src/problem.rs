//! Telling the user something went wrong.
//!
//! Murmel is used with its window closed — that is the point of a dictation
//! tool. Every failure path used to end in a toast in that window, which meant
//! the common case was a silent one: the text simply never arrived and nothing
//! said why.
//!
//! The overlay is where the user is already looking, so that is where problems
//! are reported. The window still gets its toast when it happens to be open,
//! and the log still gets the technical detail, but neither is the primary
//! channel any more.
//!
//! **The overlay is told a key, not a sentence.** Wording lives in the same
//! translation files as the rest of the UI; sending a formatted string from
//! here would either be English for everyone or duplicate the translations in
//! Rust.

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Emitter};

/// What went wrong, at the granularity a user can act on.
///
/// Deliberately coarse. "Connection refused" and "502 from the endpoint" are
/// both `RefinementFailed` as far as the overlay is concerned — the difference
/// belongs in `detail`, which is shown on demand, not in the two seconds of
/// attention a pill by the cursor gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Problem {
    /// The microphone is blocked by the operating system.
    MicrophonePermission,
    /// No usable input device.
    NoInputDevice,
    /// Recording failed for some other reason.
    RecordingFailed,
    /// The speech-to-text engine failed. The audio is kept; the history entry
    /// can be transcribed again.
    TranscriptionFailed,
    /// A model could not be loaded.
    ModelLoadFailed,
    /// The language model failed or was unreachable.
    RefinementFailed,
    /// A rewrite hotkey found nothing selected.
    NothingSelected,
    /// The text could not be delivered to the target window.
    PasteFailed,
    /// Learning from a correction did not work.
    DictionaryFailed,
}

impl Problem {
    /// Suffix of the translation key, under `problem.*` in both locale files.
    pub fn key(self) -> &'static str {
        match self {
            Problem::MicrophonePermission => "microphonePermission",
            Problem::NoInputDevice => "noInputDevice",
            Problem::RecordingFailed => "recordingFailed",
            Problem::TranscriptionFailed => "transcriptionFailed",
            Problem::ModelLoadFailed => "modelLoadFailed",
            Problem::RefinementFailed => "refinementFailed",
            Problem::NothingSelected => "nothingSelected",
            Problem::PasteFailed => "pasteFailed",
            Problem::DictionaryFailed => "dictionaryFailed",
        }
    }

    /// Whether the dictation this problem belongs to left a history entry that
    /// can be retried. It is the difference between "something broke" and
    /// "something broke, and your words are not lost".
    pub fn is_recoverable_from_history(self) -> bool {
        matches!(
            self,
            Problem::TranscriptionFailed | Problem::RefinementFailed | Problem::PasteFailed
        )
    }
}

/// What both the overlay and the settings window receive.
#[derive(Clone, Debug, Serialize, Type)]
pub struct ProblemReport {
    pub problem: Problem,
    /// Translation key suffix, so the frontend need not map the enum itself.
    pub key: &'static str,
    /// The technical reason, for the details line. Already in the log too.
    pub detail: Option<String>,
    /// Whether the history holds something to retry.
    pub recoverable: bool,
}

/// Report a problem everywhere it belongs: the log, the overlay, the window.
///
/// One call site per failure instead of three, so a new failure path cannot
/// reach the user through fewer channels than an old one by omission.
pub fn report(app: &AppHandle, problem: Problem, detail: Option<String>) {
    match &detail {
        Some(detail) => log::error!("{:?}: {}", problem, detail),
        None => log::error!("{:?}", problem),
    }

    let report = ProblemReport {
        problem,
        key: problem.key(),
        detail,
        recoverable: problem.is_recoverable_from_history(),
    };

    // The overlay first — it is the one the user can actually see while
    // dictating with the window closed.
    crate::overlay::show_problem(app, &report);

    // And the window, for whoever has it open. Same payload, so the two cannot
    // drift apart in what they say.
    let _ = app.emit("murmel-problem", report);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The frontend looks the wording up under `problem.<key>`. A key that is
    /// empty or shares its name with another would resolve to the wrong text or
    /// none at all — neither shows up as a test failure anywhere else.
    #[test]
    fn every_problem_has_its_own_key() {
        let problems = [
            Problem::MicrophonePermission,
            Problem::NoInputDevice,
            Problem::RecordingFailed,
            Problem::TranscriptionFailed,
            Problem::ModelLoadFailed,
            Problem::RefinementFailed,
            Problem::NothingSelected,
            Problem::PasteFailed,
            Problem::DictionaryFailed,
        ];

        let mut seen = std::collections::HashSet::new();
        for problem in problems {
            let key = problem.key();
            assert!(!key.is_empty(), "{problem:?} has an empty key");
            assert!(seen.insert(key), "{problem:?} reuses the key '{key}'");
        }
    }

    /// Only failures that leave a retryable history entry may promise one.
    /// Telling the user their words are safe when they are not is worse than
    /// saying nothing.
    #[test]
    fn only_recoverable_failures_promise_a_retry() {
        assert!(Problem::TranscriptionFailed.is_recoverable_from_history());
        assert!(Problem::PasteFailed.is_recoverable_from_history());

        // Nothing was recorded in these cases, so there is nothing to go back to.
        assert!(!Problem::MicrophonePermission.is_recoverable_from_history());
        assert!(!Problem::NoInputDevice.is_recoverable_from_history());
        assert!(!Problem::NothingSelected.is_recoverable_from_history());
    }
}
