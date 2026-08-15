//! Backups of the settings file.
//!
//! Two of them, for two different accidents. A rolling copy taken at startup
//! covers the setting you did not mean to change and the file that went
//! missing — it is there without anyone having thought of it beforehand. An
//! export covers the machine you no longer have.
//!
//! They differ in one deliberate way: the automatic copy is byte-for-byte, the
//! export is stripped of API keys. The automatic copy never leaves the app data
//! directory and is therefore exactly as protected as the original. An export
//! is a file someone carries around, and credentials do not belong in one —
//! keys normally live in the OS credential store (`secrets.rs`), but when that
//! store is unavailable they stay in the settings file, so an export cannot
//! assume they are absent.

use crate::settings::{AppSettings, CURRENT_SETTINGS_SCHEMA_VERSION, SETTINGS_STORE_PATH};
use chrono::{DateTime, Local};
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;

/// How many automatic backups to keep. Enough to step back past a change you
/// only noticed a few starts later, few enough to stay a rounding error on disk.
const KEEP: usize = 5;

const BACKUP_DIR: &str = "settings-backups";
const FILE_PREFIX: &str = "settings-";

/// Format version of an exported file. Separate from the settings schema
/// version: this one describes the envelope, that one what is inside it.
const EXPORT_FORMAT: u32 = 1;

/// One automatic backup, for the restore list.
#[derive(Serialize, Deserialize, Type, Debug, Clone)]
pub struct SettingsBackup {
    /// File name, and the handle `restore` takes. Never a path — a caller must
    /// not be able to reach outside the backup directory.
    pub name: String,
    /// When it was taken, formatted for display in the local timezone.
    pub taken_at: String,
}

/// The envelope an export is wrapped in, so an import can tell what it is
/// holding before it acts on it.
#[derive(Serialize, Deserialize, Debug)]
struct ExportEnvelope {
    /// Presence and value of this field is what makes a file recognisable as a
    /// Murmel settings export rather than arbitrary JSON.
    murmel_settings_export: u32,
    app_version: String,
    exported_at: String,
    settings: AppSettings,
}

fn backup_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::portable::app_state_dir(app)
        .map_err(|e| format!("No app data directory: {e}"))?
        .join(BACKUP_DIR);
    Ok(dir)
}

fn settings_file(app: &AppHandle) -> Result<PathBuf, String> {
    let stored = crate::portable::store_path(SETTINGS_STORE_PATH);

    // `store_path` is absolute in portable mode and relative to the app data
    // dir otherwise; both have to end up as a real path here.
    if stored.is_absolute() {
        return Ok(stored);
    }

    Ok(crate::portable::app_data_dir(app)
        .map_err(|e| format!("No app data directory: {e}"))?
        .join(stored))
}

/// Take a copy of the settings file, unless the newest copy is already
/// identical. Called at startup.
///
/// Backing up on every start regardless would push the useful history out of
/// the list in five launches, which is the opposite of what it is for.
pub fn rotate_on_start(app: &AppHandle) {
    if let Err(err) = rotate(app) {
        // A missing backup is not worth interrupting a launch over.
        warn!("Could not back up settings: {err}");
    }
}

fn rotate(app: &AppHandle) -> Result<(), String> {
    let source = settings_file(app)?;
    if !source.exists() {
        return Ok(()); // First run: nothing configured to lose yet.
    }

    let current = fs::read(&source).map_err(|e| format!("{}: {e}", source.display()))?;

    let dir = backup_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;

    let mut existing = backup_files(&dir)?;

    if let Some(newest) = existing.last() {
        if fs::read(newest)
            .map(|bytes| bytes == current)
            .unwrap_or(false)
        {
            debug!("Settings unchanged since the last backup");
            return Ok(());
        }
    }

    let name = format!(
        "{FILE_PREFIX}{}.json",
        Local::now().format("%Y-%m-%d_%H-%M-%S")
    );
    let target = dir.join(&name);
    fs::write(&target, &current).map_err(|e| format!("{}: {e}", target.display()))?;
    debug!("Settings backed up to {name}");

    existing.push(target);
    prune(&existing);

    Ok(())
}

/// Drop the oldest backups beyond [`KEEP`].
fn prune(sorted_oldest_first: &[PathBuf]) {
    let Some(surplus) = sorted_oldest_first.len().checked_sub(KEEP) else {
        return;
    };

    for path in sorted_oldest_first.iter().take(surplus) {
        if let Err(err) = fs::remove_file(path) {
            warn!("Could not remove old backup {}: {err}", path.display());
        }
    }
}

/// Backup files, oldest first. Sorting is by name, which the timestamp format
/// makes chronological.
fn backup_files(dir: &PathBuf) -> Result<Vec<PathBuf>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(FILE_PREFIX) && name.ends_with(".json"))
        })
        .collect();

    files.sort();
    Ok(files)
}

/// The automatic backups, newest first.
pub fn list(app: &AppHandle) -> Result<Vec<SettingsBackup>, String> {
    let dir = backup_dir(app)?;
    let mut backups: Vec<SettingsBackup> = backup_files(&dir)?
        .into_iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?.to_string();
            let taken_at = fs::metadata(&path)
                .and_then(|meta| meta.modified())
                .map(|time| {
                    DateTime::<Local>::from(time)
                        .format("%d.%m.%Y %H:%M")
                        .to_string()
                })
                .unwrap_or_default();
            Some(SettingsBackup { name, taken_at })
        })
        .collect();

    backups.reverse();
    Ok(backups)
}

/// Read a named automatic backup back into the settings file.
///
/// Takes a file name, never a path: joining caller-supplied text onto a
/// directory is how `../` reaches the rest of the disk.
pub fn read_backup(app: &AppHandle, name: &str) -> Result<AppSettings, String> {
    if name.contains(['/', '\\']) || name.contains("..") {
        return Err(format!("Not a backup name: {name}"));
    }

    let path = backup_dir(app)?.join(name);
    let raw = fs::read_to_string(&path).map_err(|e| format!("{name}: {e}"))?;

    // An automatic backup is a copy of the store file, whose settings sit under
    // a "settings" key — the same shape `get_settings` reads.
    let stored: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("{name} is not valid JSON: {e}"))?;

    let settings = stored
        .get("settings")
        .ok_or_else(|| format!("{name} holds no settings"))?;

    serde_json::from_value(settings.clone()).map_err(|e| format!("{name} is unreadable: {e}"))
}

/// Serialise the current settings for the user to keep, without credentials.
pub fn export(mut settings: AppSettings) -> Result<String, String> {
    // Not "they should already be empty" — see the module comment.
    settings.post_process_api_keys.clear();

    let envelope = ExportEnvelope {
        murmel_settings_export: EXPORT_FORMAT,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: Local::now().to_rfc3339(),
        settings,
    };

    serde_json::to_string_pretty(&envelope).map_err(|e| format!("Could not write export: {e}"))
}

/// Parse an exported file back into settings.
pub fn import(json: &str) -> Result<AppSettings, String> {
    let envelope: ExportEnvelope = serde_json::from_str(json)
        .map_err(|e| format!("This is not a Murmel settings export: {e}"))?;

    if envelope.murmel_settings_export > EXPORT_FORMAT {
        return Err(format!(
            "This export was written by a newer version of Murmel (format {}, this build reads up to {EXPORT_FORMAT}).",
            envelope.murmel_settings_export
        ));
    }

    // The same one-way street as the history database: a settings file from a
    // future schema may mean something this build would misread.
    if envelope.settings.settings_schema_version > CURRENT_SETTINGS_SCHEMA_VERSION {
        return Err(format!(
            "These settings come from a newer version of Murmel (schema {}, this build understands {CURRENT_SETTINGS_SCHEMA_VERSION}). Install the newer version to restore them.",
            envelope.settings.settings_schema_version
        ));
    }

    Ok(envelope.settings)
}

/// Carry the credentials of the running installation across a restore.
///
/// A backup holds no keys by design, so restoring one verbatim would silently
/// disconnect every configured provider.
pub fn keep_existing_keys(restored: &mut AppSettings, current: &AppSettings) {
    restored
        .post_process_api_keys
        .clone_from(&current.post_process_api_keys);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_export_never_carries_api_keys() {
        let mut settings = AppSettings::default();
        settings
            .post_process_api_keys
            .insert("openai".to_string(), "sk-very-secret".to_string());

        let json = export(settings).expect("export");

        assert!(
            !json.contains("sk-very-secret"),
            "an exported file must not contain credentials: {json}"
        );
    }

    #[test]
    fn an_export_reads_back_as_the_same_settings() {
        let mut settings = AppSettings::default();
        settings.selected_model = "parakeet-v3".to_string();

        let restored = import(&export(settings).expect("export")).expect("import");

        assert_eq!(restored.selected_model, "parakeet-v3");
    }

    #[test]
    fn a_newer_export_format_is_refused_rather_than_guessed_at() {
        let json = format!(
            r#"{{"murmel_settings_export": {}, "app_version": "9.9.9", "exported_at": "", "settings": {}}}"#,
            EXPORT_FORMAT + 1,
            serde_json::to_string(&AppSettings::default()).expect("settings")
        );

        let err = import(&json).expect_err("a newer format must be refused");
        assert!(err.contains("newer version"), "{err}");
    }

    #[test]
    fn arbitrary_json_is_not_mistaken_for_an_export() {
        assert!(import(r#"{"hello": "world"}"#).is_err());
    }

    /// The trap: a restore that faithfully reproduces the backup would wipe the
    /// keys of the installation doing the restoring.
    #[test]
    fn restoring_keeps_the_keys_of_the_running_installation() {
        let mut current = AppSettings::default();
        current
            .post_process_api_keys
            .insert("openai".to_string(), "sk-live".to_string());

        let mut restored = AppSettings::default();
        keep_existing_keys(&mut restored, &current);

        assert_eq!(
            restored
                .post_process_api_keys
                .get("openai")
                .map(String::as_str),
            Some("sk-live")
        );
    }

    #[test]
    fn a_backup_name_cannot_escape_the_backup_directory() {
        // No AppHandle needed: the check runs before any path is built.
        for name in ["../settings_store.json", "sub/settings.json", ".."] {
            assert!(
                name.contains(['/', '\\']) || name.contains(".."),
                "{name} must be rejected"
            );
        }
    }
}
