use crate::managers::model::ModelManager;
use crate::settings::{get_settings, write_settings};
use crate::settings_backup::{self, SettingsBackup};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;
use tauri::{AppHandle, State};

/// What the user needs to know after settings came back.
#[derive(Serialize, Deserialize, Type, Debug, Clone)]
pub struct RestoreReport {
    /// The transcription model the restored settings ask for, when it is not on
    /// disk. The UI offers to fetch it; nothing downloads on its own — a
    /// restore should never start a multi-gigabyte transfer behind the user's
    /// back.
    pub missing_model: Option<String>,
}

/// Apply restored settings, keeping what must not come from a backup.
fn apply(
    app: &AppHandle,
    mut restored: crate::settings::AppSettings,
    model_manager: &ModelManager,
) -> RestoreReport {
    let current = get_settings(app);
    settings_backup::keep_existing_keys(&mut restored, &current);

    let wanted = restored.selected_model.clone();
    write_settings(app, restored);

    let missing_model = model_manager
        .get_available_models()
        .iter()
        .find(|model| model.id == wanted)
        .map(|model| !model.is_downloaded)
        // A model the catalog does not know at all is just as unusable as one
        // that was never downloaded, and the name is still worth showing.
        .unwrap_or(true)
        .then_some(wanted);

    RestoreReport { missing_model }
}

/// The automatic backups taken at startup, newest first.
#[tauri::command]
#[specta::specta]
pub async fn list_settings_backups(app: AppHandle) -> Result<Vec<SettingsBackup>, String> {
    settings_backup::list(&app)
}

#[tauri::command]
#[specta::specta]
pub async fn restore_settings_backup(
    app: AppHandle,
    name: String,
    model_manager: State<'_, Arc<ModelManager>>,
) -> Result<RestoreReport, String> {
    let restored = settings_backup::read_backup(&app, &name)?;
    Ok(apply(&app, restored, &model_manager))
}

/// The current settings as a file the user can keep. Never includes API keys.
#[tauri::command]
#[specta::specta]
pub async fn export_settings(app: AppHandle) -> Result<String, String> {
    settings_backup::export(get_settings(&app))
}

#[tauri::command]
#[specta::specta]
pub async fn import_settings(
    app: AppHandle,
    json: String,
    model_manager: State<'_, Arc<ModelManager>>,
) -> Result<RestoreReport, String> {
    let restored = settings_backup::import(&json)?;
    Ok(apply(&app, restored, &model_manager))
}
