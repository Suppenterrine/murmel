use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, Utc};
use log::{debug, error, info};
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_specta::Event;

/// Database migrations for transcription history.
/// Each migration is applied in order. The library tracks which migrations
/// have been applied using SQLite's user_version pragma.
///
/// Note: For users upgrading from tauri-plugin-sql, migrate_from_tauri_plugin_sql()
/// converts the old _sqlx_migrations table tracking to the user_version pragma,
/// ensuring migrations don't re-run on existing databases.
static MIGRATIONS: &[M] = &[
    M::up(
        "CREATE TABLE IF NOT EXISTS transcription_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_name TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            saved BOOLEAN NOT NULL DEFAULT 0,
            title TEXT NOT NULL,
            transcription_text TEXT NOT NULL
        );",
    ),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_processed_text TEXT;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_process_prompt TEXT;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_process_requested BOOLEAN NOT NULL DEFAULT 0;"),
    // Metrics for the Insights view. Every value here already falls out of the
    // recording pipeline — nothing is measured that wasn't measured before, it
    // is merely kept. Existing rows get NULL and are skipped by the statistics.
    M::up("ALTER TABLE transcription_history ADD COLUMN duration_ms INTEGER;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN word_count INTEGER;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN processing_ms INTEGER;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN model_used TEXT;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN language TEXT;"),
    // Post-processing moves into its own table: one row per run, so a transcript
    // can be polished more than once and failed runs stay on record instead of
    // vanishing into a `None`. The two columns this replaces are dropped in the
    // same migration — keeping both would mean two sources of truth for one text.
    //
    // Historical rows are carried over. Their provider is unknown, but the old
    // schema conflated two different things in `post_processed_text`: an actual
    // LLM run (`post_process_requested = 1`) and a plain Chinese-variant
    // conversion. They are labelled apart rather than blurred together.
    M::up(
        "CREATE TABLE post_process_runs (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            history_id  INTEGER NOT NULL REFERENCES transcription_history(id) ON DELETE CASCADE,
            timestamp   INTEGER NOT NULL,
            provider_id TEXT NOT NULL,
            model       TEXT,
            prompt_id   TEXT,
            prompt_text TEXT,
            input_text  TEXT NOT NULL,
            output_text TEXT,
            duration_ms INTEGER,
            succeeded   BOOLEAN NOT NULL DEFAULT 0,
            error       TEXT
        );
        CREATE INDEX idx_post_process_runs_history ON post_process_runs(history_id);
        INSERT INTO post_process_runs (
            history_id, timestamp, provider_id, prompt_text,
            input_text, output_text, succeeded
        )
        SELECT
            id,
            timestamp,
            CASE WHEN post_process_requested = 1 THEN 'unknown' ELSE 'opencc' END,
            post_process_prompt,
            transcription_text,
            post_processed_text,
            1
        FROM transcription_history
        WHERE post_processed_text IS NOT NULL;
        ALTER TABLE transcription_history DROP COLUMN post_processed_text;
        ALTER TABLE transcription_history DROP COLUMN post_process_prompt;",
    ),
];

/// Provider id recorded for the Chinese-variant conversion, which is a text
/// transformation but not an LLM call. Keeping it in the same table means the
/// history has one place to look for "what happened to this text after
/// transcription"; the id keeps it distinguishable from a real post-process run.
pub const OPENCC_PROVIDER_ID: &str = "opencc";

/// The `SELECT` every history query is built on. It is shared because the five
/// call sites previously repeated the column list verbatim, so each schema
/// change meant five identical edits and any missed one failed at runtime
/// rather than at compile time.
///
/// The refinement columns come from the newest run per entry and are prefixed
/// `pp_` — without aliases, `id` and `timestamp` exist on both sides of the
/// join and the mapper would silently read the wrong one.
const ENTRY_SELECT: &str = "SELECT
        h.id, h.file_name, h.timestamp, h.saved, h.title, h.transcription_text,
        h.post_process_requested, h.duration_ms, h.word_count, h.processing_ms,
        h.model_used, h.language,
        r.id AS pp_id,
        r.timestamp AS pp_timestamp,
        r.provider_id AS pp_provider_id,
        r.model AS pp_model,
        r.prompt_id AS pp_prompt_id,
        r.prompt_text AS pp_prompt_text,
        r.input_text AS pp_input_text,
        r.output_text AS pp_output_text,
        r.duration_ms AS pp_duration_ms,
        r.succeeded AS pp_succeeded,
        r.error AS pp_error
     FROM transcription_history h
     LEFT JOIN post_process_runs r ON r.id = (
         SELECT id FROM post_process_runs
         WHERE history_id = h.id
         ORDER BY id DESC
         LIMIT 1
     )";

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct PaginatedHistory {
    pub entries: Vec<HistoryEntry>,
    pub has_more: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(tag = "action")]
pub enum HistoryUpdatePayload {
    #[serde(rename = "added")]
    Added { entry: HistoryEntry },
    #[serde(rename = "updated")]
    Updated { entry: HistoryEntry },
    #[serde(rename = "deleted")]
    Deleted { id: i64 },
    #[serde(rename = "toggled")]
    Toggled { id: i64 },
}

/// A single pass of a transcript through text refinement — an LLM call, or the
/// Chinese-variant conversion (see [`OPENCC_PROVIDER_ID`]).
///
/// Failed runs are recorded too, with `succeeded = false` and the reason in
/// `error`. That is deliberate: the share of runs that fail is the number that
/// tells you whether a local model is actually dependable.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct PostProcessRun {
    pub id: i64,
    pub history_id: i64,
    pub timestamp: i64,
    pub provider_id: String,
    pub model: Option<String>,
    pub prompt_id: Option<String>,
    pub prompt_text: Option<String>,
    /// What the model was given. Usually the raw transcript, but not
    /// necessarily — a second pass refines the first pass's output.
    pub input_text: String,
    pub output_text: Option<String>,
    pub duration_ms: Option<i64>,
    pub succeeded: bool,
    pub error: Option<String>,
}

/// Everything needed to persist a finished transcription. This is a struct
/// rather than a parameter list because the metrics are almost all `Option<i64>`
/// — as positional arguments they would be trivial to transpose, and a swapped
/// `duration_ms`/`processing_ms` pair produces plausible-looking statistics that
/// are quietly wrong forever.
#[derive(Clone, Debug, Default)]
pub struct NewHistoryEntry {
    pub file_name: String,
    pub transcription_text: String,
    pub post_process_requested: bool,
    /// Length of the recorded audio.
    pub duration_ms: Option<i64>,
    /// Words in the *raw* transcript. Counting the refined text instead would
    /// measure the language model rather than the speaker.
    pub word_count: Option<i64>,
    /// Wall time spent in the speech-to-text engine.
    pub processing_ms: Option<i64>,
    pub model_used: Option<String>,
    pub language: Option<String>,
    /// Refinement passes, in the order they ran. Persisted together with the
    /// entry. A list rather than a single value because a transcript can go
    /// through more than one pass — a variant conversion followed by an LLM,
    /// say — and the second must not erase the first.
    pub post_process: Vec<NewPostProcessRun>,
}

/// New results for an entry that is being transcribed again.
#[derive(Clone, Debug, Default)]
pub struct TranscriptionUpdate {
    pub transcription_text: String,
    pub word_count: Option<i64>,
    pub processing_ms: Option<i64>,
    pub model_used: Option<String>,
    pub language: Option<String>,
    pub post_process: Vec<NewPostProcessRun>,
}

/// A refinement run that has happened but has no database row yet.
#[derive(Clone, Debug)]
pub struct NewPostProcessRun {
    pub provider_id: String,
    pub model: Option<String>,
    pub prompt_id: Option<String>,
    pub prompt_text: Option<String>,
    pub input_text: String,
    pub output_text: Option<String>,
    pub duration_ms: Option<i64>,
    pub succeeded: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct HistoryEntry {
    pub id: i64,
    pub file_name: String,
    pub timestamp: i64,
    pub saved: bool,
    pub title: String,
    pub transcription_text: String,
    /// Whether refinement was *asked for* (which hotkey was pressed) — a
    /// property of the dictation, unlike the runs themselves.
    pub post_process_requested: bool,
    pub duration_ms: Option<i64>,
    pub word_count: Option<i64>,
    pub processing_ms: Option<i64>,
    pub model_used: Option<String>,
    pub language: Option<String>,
    /// Most recent run from `post_process_runs`, if any.
    pub last_post_process: Option<PostProcessRun>,
}

impl HistoryEntry {
    /// The text to hand to the user: the refined version when one succeeded,
    /// otherwise the raw transcript.
    pub fn display_text(&self) -> &str {
        self.last_post_process
            .as_ref()
            .and_then(|run| run.output_text.as_deref())
            .filter(|text| !text.is_empty())
            .unwrap_or(&self.transcription_text)
    }
}

pub struct HistoryManager {
    app_handle: AppHandle,
    recordings_dir: PathBuf,
    db_path: PathBuf,
}

impl HistoryManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        // Create recordings directory in app data dir
        let app_data_dir = crate::portable::app_data_dir(app_handle)?;
        let recordings_dir = app_data_dir.join("recordings");
        let db_path = app_data_dir.join("history.db");

        // Ensure recordings directory exists
        if !recordings_dir.exists() {
            fs::create_dir_all(&recordings_dir)?;
            debug!("Created recordings directory: {:?}", recordings_dir);
        }

        let manager = Self {
            app_handle: app_handle.clone(),
            recordings_dir,
            db_path,
        };

        // Initialize database and run migrations synchronously
        manager.init_database()?;

        Ok(manager)
    }

    fn init_database(&self) -> Result<()> {
        info!("Initializing database at {:?}", self.db_path);

        let mut conn = Connection::open(&self.db_path)?;

        // Handle migration from tauri-plugin-sql to rusqlite_migration
        // tauri-plugin-sql used _sqlx_migrations table, rusqlite_migration uses user_version pragma
        self.migrate_from_tauri_plugin_sql(&conn)?;

        // Create migrations object and run to latest version
        let migrations = Migrations::new(MIGRATIONS.to_vec());

        // Validate migrations in debug builds
        #[cfg(debug_assertions)]
        migrations.validate().expect("Invalid migrations");

        // Get current version before migration
        let version_before: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        debug!("Database version before migration: {}", version_before);

        // Apply any pending migrations
        migrations.to_latest(&mut conn)?;

        // Get version after migration
        let version_after: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if version_after > version_before {
            info!(
                "Database migrated from version {} to {}",
                version_before, version_after
            );
        } else {
            debug!("Database already at latest version {}", version_after);
        }

        Ok(())
    }

    /// Migrate from tauri-plugin-sql's migration tracking to rusqlite_migration's.
    /// tauri-plugin-sql used a _sqlx_migrations table, while rusqlite_migration uses
    /// SQLite's user_version pragma. This function checks if the old system was in use
    /// and sets the user_version accordingly so migrations don't re-run.
    fn migrate_from_tauri_plugin_sql(&self, conn: &Connection) -> Result<()> {
        // Check if the old _sqlx_migrations table exists
        let has_sqlx_migrations: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_sqlx_migrations {
            return Ok(());
        }

        // Check current user_version
        let current_version: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if current_version > 0 {
            // Already migrated to rusqlite_migration system
            return Ok(());
        }

        // Get the highest version from the old migrations table
        let old_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if old_version > 0 {
            info!(
                "Migrating from tauri-plugin-sql (version {}) to rusqlite_migration",
                old_version
            );

            // Set user_version to match the old migration state
            conn.pragma_update(None, "user_version", old_version)?;

            // Optionally drop the old migrations table (keeping it doesn't hurt)
            // conn.execute("DROP TABLE IF EXISTS _sqlx_migrations", [])?;

            info!(
                "Migration tracking converted: user_version set to {}",
                old_version
            );
        }

        Ok(())
    }

    fn get_connection(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)?;
        Self::apply_pragmas(&conn)?;
        Ok(conn)
    }

    /// SQLite disables foreign key enforcement by default, and it is a
    /// *per-connection* setting — not a property of the database file. Without
    /// this, `post_process_runs.history_id ... ON DELETE CASCADE` is silently
    /// inert and deleting a transcript leaves its runs behind as orphans that
    /// nothing ever cleans up.
    fn apply_pragmas(conn: &Connection) -> Result<()> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(())
    }

    fn map_history_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
        // `pp_id` is NULL exactly when the LEFT JOIN found no run, which is the
        // only thing distinguishing "never refined" from "refined and failed" —
        // a failed run is a row with `succeeded = 0`, not a missing one.
        let last_post_process = match row.get::<_, Option<i64>>("pp_id")? {
            Some(id) => Some(PostProcessRun {
                id,
                history_id: row.get("id")?,
                timestamp: row.get("pp_timestamp")?,
                provider_id: row.get("pp_provider_id")?,
                model: row.get("pp_model")?,
                prompt_id: row.get("pp_prompt_id")?,
                prompt_text: row.get("pp_prompt_text")?,
                input_text: row.get("pp_input_text")?,
                output_text: row.get("pp_output_text")?,
                duration_ms: row.get("pp_duration_ms")?,
                succeeded: row.get("pp_succeeded")?,
                error: row.get("pp_error")?,
            }),
            None => None,
        };

        Ok(HistoryEntry {
            id: row.get("id")?,
            file_name: row.get("file_name")?,
            timestamp: row.get("timestamp")?,
            saved: row.get("saved")?,
            title: row.get("title")?,
            transcription_text: row.get("transcription_text")?,
            post_process_requested: row.get("post_process_requested")?,
            duration_ms: row.get("duration_ms")?,
            word_count: row.get("word_count")?,
            processing_ms: row.get("processing_ms")?,
            model_used: row.get("model_used")?,
            language: row.get("language")?,
            last_post_process,
        })
    }

    pub fn recordings_dir(&self) -> &std::path::Path {
        &self.recordings_dir
    }

    /// Save a new history entry to the database.
    /// The WAV file should already have been written to the recordings directory.
    ///
    /// The refinement run, if there was one, is written in the same transaction:
    /// entry and run are one event to the user, and splitting them would let the
    /// UI show the raw transcript for a moment before the polished text lands.
    pub fn save_entry(&self, new_entry: NewHistoryEntry) -> Result<HistoryEntry> {
        let timestamp = Utc::now().timestamp();
        let title = self.format_timestamp_title(timestamp);

        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_process_requested,
                duration_ms,
                word_count,
                processing_ms,
                model_used,
                language
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &new_entry.file_name,
                timestamp,
                false,
                &title,
                &new_entry.transcription_text,
                new_entry.post_process_requested,
                new_entry.duration_ms,
                new_entry.word_count,
                new_entry.processing_ms,
                &new_entry.model_used,
                &new_entry.language,
            ],
        )?;
        let id = tx.last_insert_rowid();

        let mut last_post_process = None;
        for run in new_entry.post_process {
            last_post_process = Some(Self::insert_post_process_run(&tx, id, timestamp, run)?);
        }
        tx.commit()?;

        let entry = HistoryEntry {
            id,
            file_name: new_entry.file_name,
            timestamp,
            saved: false,
            title,
            transcription_text: new_entry.transcription_text,
            post_process_requested: new_entry.post_process_requested,
            duration_ms: new_entry.duration_ms,
            word_count: new_entry.word_count,
            processing_ms: new_entry.processing_ms,
            model_used: new_entry.model_used,
            language: new_entry.language,
            last_post_process,
        };

        debug!("Saved history entry with id {}", entry.id);

        self.cleanup_old_entries()?;

        // Emit typed event for real-time frontend updates
        if let Err(e) = (HistoryUpdatePayload::Added {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(entry)
    }

    /// Write one refinement run. Shared by the initial save and by later passes
    /// over an existing entry, so the row is built in exactly one place.
    fn insert_post_process_run(
        conn: &Connection,
        history_id: i64,
        timestamp: i64,
        run: NewPostProcessRun,
    ) -> Result<PostProcessRun> {
        conn.execute(
            "INSERT INTO post_process_runs (
                history_id, timestamp, provider_id, model, prompt_id,
                prompt_text, input_text, output_text, duration_ms, succeeded, error
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                history_id,
                timestamp,
                &run.provider_id,
                &run.model,
                &run.prompt_id,
                &run.prompt_text,
                &run.input_text,
                &run.output_text,
                run.duration_ms,
                run.succeeded,
                &run.error,
            ],
        )?;

        Ok(PostProcessRun {
            id: conn.last_insert_rowid(),
            history_id,
            timestamp,
            provider_id: run.provider_id,
            model: run.model,
            prompt_id: run.prompt_id,
            prompt_text: run.prompt_text,
            input_text: run.input_text,
            output_text: run.output_text,
            duration_ms: run.duration_ms,
            succeeded: run.succeeded,
            error: run.error,
        })
    }

    /// Update an existing history entry with new transcription results (used by retry).
    ///
    /// The metrics are rewritten along with the text: a retry may well use a
    /// different model and will certainly take a different amount of time, and
    /// leaving the previous run's numbers in place would attribute them to a
    /// transcript that no longer exists. The audio is unchanged, so
    /// `duration_ms` stays.
    pub fn update_transcription(
        &self,
        id: i64,
        update: TranscriptionUpdate,
    ) -> Result<HistoryEntry> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;
        let updated = tx.execute(
            "UPDATE transcription_history
             SET transcription_text = ?1,
                 word_count = ?2,
                 processing_ms = ?3,
                 model_used = ?4,
                 language = ?5
             WHERE id = ?6",
            params![
                &update.transcription_text,
                update.word_count,
                update.processing_ms,
                &update.model_used,
                &update.language,
                id,
            ],
        )?;

        if updated == 0 {
            return Err(anyhow!("History entry {} not found", id));
        }

        let now = Utc::now().timestamp();
        for run in update.post_process {
            Self::insert_post_process_run(&tx, id, now, run)?;
        }
        tx.commit()?;

        let entry = conn.query_row(
            &format!("{ENTRY_SELECT} WHERE h.id = ?1"),
            params![id],
            Self::map_history_entry,
        )?;

        debug!("Updated transcription for history entry {}", id);

        if let Err(e) = (HistoryUpdatePayload::Updated {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(entry)
    }

    pub fn cleanup_old_entries(&self) -> Result<()> {
        let retention_period = crate::settings::get_recording_retention_period(&self.app_handle);

        match retention_period {
            crate::settings::RecordingRetentionPeriod::Never => {
                // Don't delete anything
                Ok(())
            }
            crate::settings::RecordingRetentionPeriod::PreserveLimit => {
                // Use the old count-based logic with history_limit
                let limit = crate::settings::get_history_limit(&self.app_handle);
                self.cleanup_by_count(limit)
            }
            _ => {
                // Use time-based logic
                self.cleanup_by_time(retention_period)
            }
        }
    }

    fn delete_entries_and_files(&self, entries: &[(i64, String)]) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let conn = self.get_connection()?;
        let mut deleted_count = 0;

        for (id, file_name) in entries {
            // Delete database entry
            conn.execute(
                "DELETE FROM transcription_history WHERE id = ?1",
                params![id],
            )?;

            // Delete WAV file
            let file_path = self.recordings_dir.join(file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete WAV file {}: {}", file_name, e);
                } else {
                    debug!("Deleted old WAV file: {}", file_name);
                    deleted_count += 1;
                }
            }
        }

        Ok(deleted_count)
    }

    fn cleanup_by_count(&self, limit: usize) -> Result<()> {
        let conn = self.get_connection()?;

        // Get all entries that are not saved, ordered by timestamp desc
        let mut stmt = conn.prepare(
            "SELECT id, file_name FROM transcription_history WHERE saved = 0 ORDER BY timestamp DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>("id")?, row.get::<_, String>("file_name")?))
        })?;

        let mut entries: Vec<(i64, String)> = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        if entries.len() > limit {
            let entries_to_delete = &entries[limit..];
            let deleted_count = self.delete_entries_and_files(entries_to_delete)?;

            if deleted_count > 0 {
                debug!("Cleaned up {} old history entries by count", deleted_count);
            }
        }

        Ok(())
    }

    fn cleanup_by_time(
        &self,
        retention_period: crate::settings::RecordingRetentionPeriod,
    ) -> Result<()> {
        let conn = self.get_connection()?;

        // Calculate cutoff timestamp (current time minus retention period)
        let now = Utc::now().timestamp();
        let cutoff_timestamp = match retention_period {
            crate::settings::RecordingRetentionPeriod::Days3 => now - (3 * 24 * 60 * 60), // 3 days in seconds
            crate::settings::RecordingRetentionPeriod::Weeks2 => now - (2 * 7 * 24 * 60 * 60), // 2 weeks in seconds
            crate::settings::RecordingRetentionPeriod::Months3 => now - (3 * 30 * 24 * 60 * 60), // 3 months in seconds (approximate)
            _ => unreachable!("Should not reach here"),
        };

        // Get all unsaved entries older than the cutoff timestamp
        let mut stmt = conn.prepare(
            "SELECT id, file_name FROM transcription_history WHERE saved = 0 AND timestamp < ?1",
        )?;

        let rows = stmt.query_map(params![cutoff_timestamp], |row| {
            Ok((row.get::<_, i64>("id")?, row.get::<_, String>("file_name")?))
        })?;

        let mut entries_to_delete: Vec<(i64, String)> = Vec::new();
        for row in rows {
            entries_to_delete.push(row?);
        }

        let deleted_count = self.delete_entries_and_files(&entries_to_delete)?;

        if deleted_count > 0 {
            debug!(
                "Cleaned up {} old history entries based on retention period",
                deleted_count
            );
        }

        Ok(())
    }

    pub async fn get_history_entries(
        &self,
        cursor: Option<i64>,
        limit: Option<usize>,
    ) -> Result<PaginatedHistory> {
        let conn = self.get_connection()?;
        let limit = limit.map(|l| l.min(100));

        let mut entries: Vec<HistoryEntry> = match (cursor, limit) {
            (Some(cursor_id), Some(lim)) => {
                let fetch_count = (lim + 1) as i64;
                let mut stmt = conn.prepare(&format!(
                    "{ENTRY_SELECT} WHERE h.id < ?1 ORDER BY h.id DESC LIMIT ?2"
                ))?;
                let result = stmt
                    .query_map(params![cursor_id, fetch_count], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
            (None, Some(lim)) => {
                let fetch_count = (lim + 1) as i64;
                let mut stmt =
                    conn.prepare(&format!("{ENTRY_SELECT} ORDER BY h.id DESC LIMIT ?1"))?;
                let result = stmt
                    .query_map(params![fetch_count], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
            (_, None) => {
                let mut stmt = conn.prepare(&format!("{ENTRY_SELECT} ORDER BY h.id DESC"))?;
                let result = stmt
                    .query_map([], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
        };

        let has_more = limit.is_some_and(|lim| entries.len() > lim);
        if has_more {
            entries.pop();
        }

        Ok(PaginatedHistory { entries, has_more })
    }

    #[cfg(test)]
    fn get_latest_entry_with_conn(conn: &Connection) -> Result<Option<HistoryEntry>> {
        let mut stmt =
            conn.prepare(&format!("{ENTRY_SELECT} ORDER BY h.timestamp DESC LIMIT 1"))?;

        let entry = stmt.query_row([], Self::map_history_entry).optional()?;
        Ok(entry)
    }

    /// Get the latest entry with non-empty transcription text.
    pub fn get_latest_completed_entry(&self) -> Result<Option<HistoryEntry>> {
        let conn = self.get_connection()?;
        Self::get_latest_completed_entry_with_conn(&conn)
    }

    fn get_latest_completed_entry_with_conn(conn: &Connection) -> Result<Option<HistoryEntry>> {
        let mut stmt = conn.prepare(&format!(
            "{ENTRY_SELECT} WHERE h.transcription_text != '' ORDER BY h.timestamp DESC LIMIT 1"
        ))?;

        let entry = stmt.query_row([], Self::map_history_entry).optional()?;
        Ok(entry)
    }

    pub async fn toggle_saved_status(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;

        // Get current saved status
        let current_saved: bool = conn.query_row(
            "SELECT saved FROM transcription_history WHERE id = ?1",
            params![id],
            |row| row.get("saved"),
        )?;

        let new_saved = !current_saved;

        conn.execute(
            "UPDATE transcription_history SET saved = ?1 WHERE id = ?2",
            params![new_saved, id],
        )?;

        debug!("Toggled saved status for entry {}: {}", id, new_saved);

        // Emit history updated event
        if let Err(e) = (HistoryUpdatePayload::Toggled { id }).emit(&self.app_handle) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    pub fn get_audio_file_path(&self, file_name: &str) -> PathBuf {
        self.recordings_dir.join(file_name)
    }

    pub async fn get_entry_by_id(&self, id: i64) -> Result<Option<HistoryEntry>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(&format!("{ENTRY_SELECT} WHERE h.id = ?1"))?;

        let entry = stmt.query_row([id], Self::map_history_entry).optional()?;

        Ok(entry)
    }

    pub async fn delete_entry(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;

        // Get the entry to find the file name
        if let Some(entry) = self.get_entry_by_id(id).await? {
            // Delete the audio file first
            let file_path = self.get_audio_file_path(&entry.file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete audio file {}: {}", entry.file_name, e);
                    // Continue with database deletion even if file deletion fails
                }
            }
        }

        // Delete from database
        conn.execute(
            "DELETE FROM transcription_history WHERE id = ?1",
            params![id],
        )?;

        debug!("Deleted history entry with id: {}", id);

        // Emit history updated event
        if let Err(e) = (HistoryUpdatePayload::Deleted { id }).emit(&self.app_handle) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    fn format_timestamp_title(&self, timestamp: i64) -> String {
        if let Some(utc_datetime) = DateTime::from_timestamp(timestamp, 0) {
            // Convert UTC to local timezone
            let local_datetime = utc_datetime.with_timezone(&Local);
            local_datetime.format("%B %e, %Y - %l:%M%p").to_string()
        } else {
            format!("Recording {}", timestamp)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};

    /// Build the schema by running the real migrations rather than a
    /// hand-written `CREATE TABLE`. The copy that used to live here had to be
    /// edited in lockstep with `MIGRATIONS`, and a mismatch surfaced as a
    /// puzzling runtime error instead of a failing test. Now the migrations are
    /// themselves covered by every test in this module.
    fn setup_conn() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        Migrations::new(MIGRATIONS.to_vec())
            .to_latest(&mut conn)
            .expect("run migrations");
        HistoryManager::apply_pragmas(&conn).expect("apply pragmas");
        conn
    }

    fn insert_entry(conn: &Connection, timestamp: i64, text: &str, post_processed: Option<&str>) {
        conn.execute(
            "INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_process_requested
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                format!("murmel-{}.wav", timestamp),
                timestamp,
                false,
                format!("Recording {}", timestamp),
                text,
                post_processed.is_some(),
            ],
        )
        .expect("insert history entry");

        if let Some(output) = post_processed {
            let history_id = conn.last_insert_rowid();
            HistoryManager::insert_post_process_run(
                conn,
                history_id,
                timestamp,
                NewPostProcessRun {
                    provider_id: "custom".to_string(),
                    model: Some("qwen3:4b".to_string()),
                    prompt_id: None,
                    prompt_text: None,
                    input_text: text.to_string(),
                    output_text: Some(output.to_string()),
                    duration_ms: Some(42),
                    succeeded: true,
                    error: None,
                },
            )
            .expect("insert post process run");
        }
    }

    #[test]
    fn get_latest_entry_returns_none_when_empty() {
        let conn = setup_conn();
        let entry = HistoryManager::get_latest_entry_with_conn(&conn).expect("fetch latest entry");
        assert!(entry.is_none());
    }

    #[test]
    fn get_latest_entry_returns_newest_entry() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "first", None);
        insert_entry(&conn, 200, "second", Some("processed"));

        let entry = HistoryManager::get_latest_entry_with_conn(&conn)
            .expect("fetch latest entry")
            .expect("entry exists");

        assert_eq!(entry.timestamp, 200);
        assert_eq!(entry.transcription_text, "second");
        let run = entry.last_post_process.expect("refinement run present");
        assert_eq!(run.output_text.as_deref(), Some("processed"));
        assert_eq!(run.model.as_deref(), Some("qwen3:4b"));
        assert!(run.succeeded);
    }

    #[test]
    fn get_latest_completed_entry_skips_empty_entries() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "completed", None);
        insert_entry(&conn, 200, "", None);

        let entry = HistoryManager::get_latest_completed_entry_with_conn(&conn)
            .expect("fetch latest completed entry")
            .expect("completed entry exists");

        assert_eq!(entry.timestamp, 100);
        assert_eq!(entry.transcription_text, "completed");
        assert!(entry.last_post_process.is_none());
    }

    /// The join must pick the newest run, not an arbitrary one — otherwise
    /// refining a dictation a second time would leave the UI showing the first
    /// result.
    #[test]
    fn latest_run_wins_when_an_entry_was_refined_twice() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "raw", Some("first pass"));

        HistoryManager::insert_post_process_run(
            &conn,
            1,
            150,
            NewPostProcessRun {
                provider_id: "custom".to_string(),
                model: None,
                prompt_id: None,
                prompt_text: None,
                input_text: "first pass".to_string(),
                output_text: Some("second pass".to_string()),
                duration_ms: None,
                succeeded: true,
                error: None,
            },
        )
        .expect("insert second run");

        let entry = HistoryManager::get_latest_entry_with_conn(&conn)
            .expect("fetch latest entry")
            .expect("entry exists");

        let run = entry
            .last_post_process
            .as_ref()
            .expect("refinement run present");
        assert_eq!(run.output_text.as_deref(), Some("second pass"));
        assert_eq!(entry.display_text(), "second pass");
    }

    /// A failed run is recorded, but must not be presented as the transcript.
    #[test]
    fn failed_run_is_recorded_without_replacing_the_transcript() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "raw", None);

        HistoryManager::insert_post_process_run(
            &conn,
            1,
            100,
            NewPostProcessRun {
                provider_id: "custom".to_string(),
                model: Some("qwen3:4b".to_string()),
                prompt_id: None,
                prompt_text: None,
                input_text: "raw".to_string(),
                output_text: None,
                duration_ms: Some(1200),
                succeeded: false,
                error: Some("connection refused".to_string()),
            },
        )
        .expect("insert failed run");

        let entry = HistoryManager::get_latest_entry_with_conn(&conn)
            .expect("fetch latest entry")
            .expect("entry exists");

        let run = entry.last_post_process.as_ref().expect("run present");
        assert!(!run.succeeded);
        assert_eq!(run.error.as_deref(), Some("connection refused"));
        assert_eq!(entry.display_text(), "raw");
    }

    /// `ON DELETE CASCADE` only works when foreign keys are enabled on the
    /// connection — SQLite has them off by default, per connection.
    #[test]
    fn deleting_an_entry_removes_its_runs() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "raw", Some("processed"));

        conn.execute("DELETE FROM transcription_history WHERE id = 1", [])
            .expect("delete entry");

        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM post_process_runs", [], |row| {
                row.get(0)
            })
            .expect("count runs");
        assert_eq!(remaining, 0);
    }

    /// Rows written under the old schema keep their refined text, and the two
    /// things the old column conflated stay distinguishable.
    #[test]
    fn legacy_rows_are_carried_into_the_runs_table() {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");

        // Migrate only as far as the old schema, then write a row the way the
        // previous version would have.
        Migrations::new(MIGRATIONS[..4].to_vec())
            .to_latest(&mut conn)
            .expect("run legacy migrations");
        conn.execute(
            "INSERT INTO transcription_history (
                file_name, timestamp, saved, title, transcription_text,
                post_processed_text, post_process_prompt, post_process_requested
            ) VALUES ('a.wav', 100, 0, 'Recording', 'raw', 'polished', 'clean it up', 1)",
            [],
        )
        .expect("insert legacy llm row");
        conn.execute(
            "INSERT INTO transcription_history (
                file_name, timestamp, saved, title, transcription_text,
                post_processed_text, post_process_requested
            ) VALUES ('b.wav', 200, 0, 'Recording', '简体', '簡體', 0)",
            [],
        )
        .expect("insert legacy conversion row");

        Migrations::new(MIGRATIONS.to_vec())
            .to_latest(&mut conn)
            .expect("run remaining migrations");

        let mut stmt = conn
            .prepare("SELECT provider_id, prompt_text, output_text FROM post_process_runs ORDER BY history_id")
            .expect("prepare");
        let runs: Vec<(String, Option<String>, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("query runs")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("collect runs");

        assert_eq!(
            runs,
            vec![
                (
                    "unknown".to_string(),
                    Some("clean it up".to_string()),
                    Some("polished".to_string())
                ),
                (
                    OPENCC_PROVIDER_ID.to_string(),
                    None,
                    Some("簡體".to_string())
                ),
            ]
        );
    }
}
