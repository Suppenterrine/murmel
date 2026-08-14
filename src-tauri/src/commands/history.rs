use crate::actions::process_transcription_output;
use crate::managers::{
    history::{
        EntryKind, HistoryEntry, HistoryManager, PaginatedHistory, TranscriptionUpdate, UsageRow,
        UsageSummary,
    },
    transcription::TranscriptionManager,
};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub async fn get_history_entries(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    cursor: Option<i64>,
    limit: Option<usize>,
) -> Result<PaginatedHistory, String> {
    history_manager
        .get_history_entries(cursor, limit)
        .await
        .map_err(|e| e.to_string())
}

/// Search the history, optionally limited to saved entries.
///
/// Separate from `get_history_entries` rather than an extra parameter there:
/// that one pages through the history by cursor, this one ranks by relevance,
/// and mixing the two would mean a cursor that means different things depending
/// on whether a search term is present.
#[tauri::command]
#[specta::specta]
pub async fn search_history_entries(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    query: Option<String>,
    only_saved: bool,
    kind: Option<EntryKind>,
    limit: Option<usize>,
) -> Result<Vec<HistoryEntry>, String> {
    history_manager
        .search_history_entries(query, only_saved, kind, limit)
        .await
        .map_err(|e| e.to_string())
}

/// Typing speed the time-saved estimate is measured against.
///
/// 40 words per minute is a common figure for someone typing prose they are
/// composing as they go — not a touch-typist copying text, which is the number
/// usually quoted and would flatter the comparison. Murmel_Northstar.md §6.1.
const TYPING_WPM: f64 = 40.0;

#[tauri::command]
#[specta::specta]
pub async fn get_usage_summary(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
) -> Result<UsageSummary, String> {
    history_manager
        .usage_summary(TYPING_WPM)
        .map_err(|e| e.to_string())
}

/// All statistics rows, for the user to take with them.
#[tauri::command]
#[specta::specta]
pub async fn export_usage_stats(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
) -> Result<Vec<UsageRow>, String> {
    history_manager.usage_rows().map_err(|e| e.to_string())
}

/// Erase the statistics without touching the history.
#[tauri::command]
#[specta::specta]
pub async fn clear_usage_stats(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
) -> Result<(), String> {
    history_manager
        .clear_usage_stats()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Save a corrected transcript and report what the dictionary could learn.
///
/// The suggestions are returned, not applied: a correction can contain a typo
/// of its own, and a dictionary entry silently taught from one would then bend
/// every future dictation towards it. The user confirms.
#[tauri::command]
#[specta::specta]
pub async fn correct_history_entry(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
    corrected_text: String,
) -> Result<Vec<String>, String> {
    let entry = history_manager
        .get_entry_by_id(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("History entry {} not found", id))?;

    let known = crate::settings::get_settings(&app).custom_words;
    let suggestions = crate::audio_toolkit::text::suggest_dictionary_entries(
        &entry.transcription_text,
        &corrected_text,
        &known,
    );

    history_manager
        .update_transcription_text(id, &corrected_text)
        .map_err(|e| e.to_string())?;

    Ok(suggestions)
}

/// Average words per dictation, or `null` while there is too little to average.
///
/// Feeds the cost estimate in the model picker: a price per million tokens
/// answers a question nobody has, a price per dictation answers the one they do.
#[tauri::command]
#[specta::specta]
pub async fn get_average_dictation_words(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
) -> Result<Option<f64>, String> {
    history_manager
        .average_word_count()
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_history_entry_saved(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    history_manager
        .toggle_saved_status(id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_audio_file_path(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    file_name: String,
) -> Result<String, String> {
    // A rewrite has no recording. Without this, the empty name would resolve to
    // the recordings directory itself and the player would be handed a folder.
    if file_name.is_empty() {
        return Err("This entry has no recording".to_string());
    }

    let path = history_manager.get_audio_file_path(&file_name);
    path.to_str()
        .ok_or_else(|| "Invalid file path".to_string())
        .map(|s| s.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_history_entry(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    history_manager
        .delete_entry(id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn retry_history_entry_transcription(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
    id: i64,
) -> Result<(), String> {
    let entry = history_manager
        .get_entry_by_id(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("History entry {} not found", id))?;

    if entry.file_name.is_empty() {
        return Err("This entry has no recording to transcribe again".to_string());
    }

    let audio_path = history_manager.get_audio_file_path(&entry.file_name);
    let samples = crate::audio_toolkit::read_wav_samples(&audio_path)
        .map_err(|e| format!("Failed to load audio: {}", e))?;

    if samples.is_empty() {
        return Err("Recording has no audio samples".to_string());
    }

    transcription_manager.initiate_model_load();

    let tm = Arc::clone(&transcription_manager);
    // Timed like the live path, so a retry's numbers are comparable with the
    // original run's rather than being left over from it.
    let started = std::time::Instant::now();
    let transcription = tauri::async_runtime::spawn_blocking(move || tm.transcribe(samples))
        .await
        .map_err(|e| format!("Transcription task panicked: {}", e))?
        .map_err(|e| e.to_string())?;
    let processing_ms = started.elapsed().as_millis() as i64;

    if transcription.is_empty() {
        return Err("Recording contains no speech".to_string());
    }

    let processed =
        process_transcription_output(&app, &transcription, entry.post_process_requested).await;
    let word_count = transcription.split_whitespace().count() as i64;

    history_manager
        .update_transcription(
            id,
            TranscriptionUpdate {
                transcription_text: transcription,
                word_count: Some(word_count),
                processing_ms: Some(processing_ms),
                model_used: Some(crate::settings::get_settings(&app).selected_model),
                language: Some(processed.effective_language),
                post_process: processed.post_process,
            },
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Run refinement over an entry again, after a failed attempt.
///
/// Nothing is pasted: the user is looking at the history, not at the window the
/// text was meant for. The result lands on the entry, where it can be read and
/// copied.
#[tauri::command]
#[specta::specta]
pub async fn retry_history_entry_refinement(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    let entry = history_manager
        .get_entry_by_id(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("History entry {} not found", id))?;

    let run = crate::actions::retry_refinement(&app, &entry).await?;

    history_manager
        .append_post_process_run(id, run)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn update_history_limit(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    limit: usize,
) -> Result<(), String> {
    let mut settings = crate::settings::get_settings(&app);
    settings.history_limit = limit;
    crate::settings::write_settings(&app, settings);

    history_manager
        .cleanup_old_entries()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_recording_retention_period(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    period: String,
) -> Result<(), String> {
    use crate::settings::RecordingRetentionPeriod;

    let retention_period = match period.as_str() {
        "never" => RecordingRetentionPeriod::Never,
        "preserve_limit" => RecordingRetentionPeriod::PreserveLimit,
        "days3" => RecordingRetentionPeriod::Days3,
        "weeks2" => RecordingRetentionPeriod::Weeks2,
        "months3" => RecordingRetentionPeriod::Months3,
        _ => return Err(format!("Invalid retention period: {}", period)),
    };

    let mut settings = crate::settings::get_settings(&app);
    settings.recording_retention_period = retention_period;
    crate::settings::write_settings(&app, settings);

    history_manager
        .cleanup_old_entries()
        .map_err(|e| e.to_string())?;

    Ok(())
}
