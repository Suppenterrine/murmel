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
    // Usage statistics outlive the history.
    //
    // The metric columns added above were attached to `transcription_history`,
    // which `cleanup_by_count` deletes rows from — with a default limit of five
    // entries. The statistics of §6.3 could therefore never have covered more
    // than the last five dictations.
    //
    // Splitting them apart is what §6.3 asks for anyway ("reset statistics
    // without losing the history — and vice versa"), and it has a pleasant
    // consequence: this table holds no transcript, no prompt and no file name,
    // only numbers, model names and timestamps. It is unobjectionable to keep
    // for years, precisely because it cannot say *what* was dictated.
    //
    // `history_id` deliberately carries no foreign key. A reference with
    // ON DELETE CASCADE would drag these rows out with the history — exactly
    // what this table exists to prevent — so it dangles once the entry is gone,
    // and is only used to attribute a later retry to the right row.
    M::up(
        "CREATE TABLE usage_stats (
            id                     INTEGER PRIMARY KEY AUTOINCREMENT,
            history_id             INTEGER,
            timestamp              INTEGER NOT NULL,
            duration_ms            INTEGER,
            word_count             INTEGER,
            processing_ms          INTEGER,
            model_used             TEXT,
            language               TEXT,
            post_process_requested  BOOLEAN NOT NULL DEFAULT 0,
            post_process_provider  TEXT,
            post_process_model     TEXT,
            post_process_ms        INTEGER,
            post_process_succeeded BOOLEAN
        );
        CREATE INDEX idx_usage_stats_timestamp ON usage_stats(timestamp);
        CREATE UNIQUE INDEX idx_usage_stats_history ON usage_stats(history_id);
        INSERT INTO usage_stats (
            history_id, timestamp, duration_ms, word_count, processing_ms,
            model_used, language, post_process_requested
        )
        SELECT
            id, timestamp, duration_ms, word_count, processing_ms,
            model_used, language, post_process_requested
        FROM transcription_history;
        ALTER TABLE transcription_history DROP COLUMN duration_ms;
        ALTER TABLE transcription_history DROP COLUMN word_count;
        ALTER TABLE transcription_history DROP COLUMN processing_ms;
        ALTER TABLE transcription_history DROP COLUMN model_used;
        ALTER TABLE transcription_history DROP COLUMN language;",
    ),
    // Full-text search over the transcripts.
    //
    // An external-content index: fts5 stores only the inverted index and reads
    // the text from `transcription_history`, so the transcripts are not held
    // twice. The price is that the triggers below have to be right — an
    // external-content index does not notice changes on its own, and a missed
    // delete leaves a phantom that matches searches forever.
    M::up(
        "CREATE VIRTUAL TABLE transcription_search USING fts5(
            transcription_text,
            content='transcription_history',
            content_rowid='id',
            tokenize='unicode61 remove_diacritics 2'
        );
        INSERT INTO transcription_search(rowid, transcription_text)
            SELECT id, transcription_text FROM transcription_history;
        CREATE TRIGGER transcription_history_ai AFTER INSERT ON transcription_history BEGIN
            INSERT INTO transcription_search(rowid, transcription_text)
            VALUES (new.id, new.transcription_text);
        END;
        CREATE TRIGGER transcription_history_ad AFTER DELETE ON transcription_history BEGIN
            INSERT INTO transcription_search(transcription_search, rowid, transcription_text)
            VALUES ('delete', old.id, old.transcription_text);
        END;
        CREATE TRIGGER transcription_history_au AFTER UPDATE ON transcription_history BEGIN
            INSERT INTO transcription_search(transcription_search, rowid, transcription_text)
            VALUES ('delete', old.id, old.transcription_text);
            INSERT INTO transcription_search(rowid, transcription_text)
            VALUES (new.id, new.transcription_text);
        END;",
    ),
    // Not every entry is a dictation any more: text that already existed
    // somewhere can be rewritten, either through a spoken instruction or by
    // running a preset over a selection.
    //
    // The column is added to *both* tables on purpose. `usage_stats` outlives
    // the transcript it describes (see the migration above), so it has to know
    // its own kind rather than read it from a row that may already be gone.
    //
    // Everything recorded so far was dictated, so the default is right
    // retroactively and no backfill is needed.
    M::up(
        "ALTER TABLE transcription_history ADD COLUMN kind TEXT NOT NULL DEFAULT 'dictation';
         ALTER TABLE usage_stats ADD COLUMN kind TEXT NOT NULL DEFAULT 'dictation';",
    ),
];

/// Where an entry's recording lives, or `None` when it has none.
///
/// A rewrite starts from text that was already on screen, so there is nothing to
/// record and `file_name` is empty. That empty name must not be joined onto the
/// recordings directory: the result is the directory itself, which exists — so
/// an `exists()` check waves it through to `remove_file` or to the audio player.
fn recording_path(recordings_dir: &std::path::Path, file_name: &str) -> Option<PathBuf> {
    if file_name.is_empty() {
        return None;
    }
    Some(recordings_dir.join(file_name))
}

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
        h.post_process_requested, h.kind,
        s.duration_ms, s.word_count, s.processing_ms, s.model_used, s.language,
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
     LEFT JOIN usage_stats s ON s.history_id = h.id
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

/// What produced an entry.
///
/// Stored as text rather than an integer so the database stays readable without
/// a lookup table, and so an unknown value from a newer version degrades to
/// "treat it as a dictation" instead of silently meaning something else.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    /// Spoken from scratch. Everything before 0.17.0 is this.
    #[default]
    Dictation,
    /// Existing text, rewritten according to a spoken instruction. The
    /// transcript is the *instruction*, not the result.
    Command,
    /// Existing text, run through a preset without speaking.
    Rewrite,
}

impl EntryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EntryKind::Dictation => "dictation",
            EntryKind::Command => "command",
            EntryKind::Rewrite => "rewrite",
        }
    }

    /// Anything unrecognised counts as a dictation — a row written by a future
    /// version should still show up in the history rather than fail the query.
    fn from_str(value: &str) -> Self {
        match value {
            "command" => EntryKind::Command,
            "rewrite" => EntryKind::Rewrite,
            _ => EntryKind::Dictation,
        }
    }
}

/// Words dictated on one calendar day, local time.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct DailyWords {
    /// `YYYY-MM-DD`.
    pub day: String,
    pub words: i64,
}

/// How much a given model was used, and how fast it was.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct ModelUsage {
    pub model: String,
    pub dictations: i64,
    pub average_processing_ms: f64,
}

/// The numbers behind the Insights view.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct UsageSummary {
    pub dictations: i64,
    pub words: i64,
    /// Time spent speaking.
    pub duration_ms: i64,
    /// Time the speech-to-text engine spent.
    pub processing_ms: i64,
    pub refined: i64,
    /// Refinement attempts that failed — the number that says whether a local
    /// model is dependable.
    pub refinement_failures: i64,
    /// Estimated time saved against typing, never negative.
    pub saved_ms: i64,
    /// Hour of day (0–23) with the most dictations, if there are any.
    pub busiest_hour: Option<u32>,
    /// Up to 30 days, oldest first.
    pub per_day: Vec<DailyWords>,
    pub models: Vec<ModelUsage>,
    /// Existing text that was rewritten rather than dictated. Counted apart
    /// from every number above, which all describe dictating.
    pub rewrites: i64,
    /// Of those, the ones steered by a spoken instruction (Command Mode).
    pub spoken_commands: i64,
}

/// One statistics row, for export.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct UsageRow {
    pub timestamp: i64,
    pub kind: EntryKind,
    pub duration_ms: Option<i64>,
    pub word_count: Option<i64>,
    pub processing_ms: Option<i64>,
    pub model_used: Option<String>,
    pub language: Option<String>,
    pub post_process_requested: bool,
    pub post_process_provider: Option<String>,
    pub post_process_model: Option<String>,
    pub post_process_ms: Option<i64>,
    pub post_process_succeeded: Option<bool>,
}

/// Everything needed to persist a finished transcription. This is a struct
/// rather than a parameter list because the metrics are almost all `Option<i64>`
/// — as positional arguments they would be trivial to transpose, and a swapped
/// `duration_ms`/`processing_ms` pair produces plausible-looking statistics that
/// are quietly wrong forever.
#[derive(Clone, Debug, Default)]
pub struct NewHistoryEntry {
    /// Empty when there is no recording — a rewrite starts from text that was
    /// already on screen, so there is nothing to save as audio.
    pub file_name: String,
    /// For a dictation and a rewrite this is the text itself; for a Command Mode
    /// entry it is the spoken *instruction*, and the text it applied to is in
    /// the refinement run's `input_text`.
    pub transcription_text: String,
    pub post_process_requested: bool,
    pub kind: EntryKind,
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
    /// What produced this entry. The history shows dictations and rewrites side
    /// by side, and without this "make it shorter" reads like a pointless
    /// three-word dictation.
    pub kind: EntryKind,
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
            kind: EntryKind::from_str(&row.get::<_, String>("kind")?),
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
    pub fn save_entry(&self, mut new_entry: NewHistoryEntry) -> Result<HistoryEntry> {
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
                kind
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &new_entry.file_name,
                timestamp,
                false,
                &title,
                &new_entry.transcription_text,
                new_entry.post_process_requested,
                new_entry.kind.as_str(),
            ],
        )?;
        let id = tx.last_insert_rowid();

        // Taken out rather than iterated in place: the statistics row below
        // still needs the rest of `new_entry`.
        let mut last_post_process = None;
        for run in std::mem::take(&mut new_entry.post_process) {
            last_post_process = Some(Self::insert_post_process_run(&tx, id, timestamp, run)?);
        }

        // Written in the same transaction, but into a table the cleanup never
        // touches — this row survives the transcript it describes.
        Self::insert_usage_stats(&tx, id, timestamp, &new_entry, last_post_process.as_ref())?;

        tx.commit()?;

        let entry = HistoryEntry {
            id,
            file_name: new_entry.file_name,
            timestamp,
            saved: false,
            title,
            transcription_text: new_entry.transcription_text,
            post_process_requested: new_entry.post_process_requested,
            kind: new_entry.kind,
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

    /// Record the numbers for one dictation, without any of its text.
    ///
    /// Of several refinement passes only the last is summarised: the questions
    /// this table answers — how often refinement is used, how often it fails,
    /// which model it runs on — do not need every intermediate pass, and the
    /// full sequence is in `post_process_runs` for as long as the entry lives.
    fn insert_usage_stats(
        conn: &Connection,
        history_id: i64,
        timestamp: i64,
        entry: &NewHistoryEntry,
        last_run: Option<&PostProcessRun>,
    ) -> Result<()> {
        conn.execute(
            "INSERT INTO usage_stats (
                history_id, timestamp, kind, duration_ms, word_count, processing_ms,
                model_used, language, post_process_requested,
                post_process_provider, post_process_model, post_process_ms,
                post_process_succeeded
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                history_id,
                timestamp,
                entry.kind.as_str(),
                entry.duration_ms,
                entry.word_count,
                entry.processing_ms,
                &entry.model_used,
                &entry.language,
                entry.post_process_requested,
                last_run.map(|run| &run.provider_id),
                last_run.and_then(|run| run.model.as_ref()),
                last_run.and_then(|run| run.duration_ms),
                last_run.map(|run| run.succeeded),
            ],
        )?;

        Ok(())
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

    /// Replace a transcript with the user's corrected version.
    ///
    /// Only the text: the metrics describe what was *dictated* — how long it
    /// took to speak and to recognise — and correcting a word afterwards does
    /// not change any of that. Rewriting the word count here would quietly
    /// distort the speaking pace in the statistics.
    pub fn update_transcription_text(&self, id: i64, text: &str) -> Result<HistoryEntry> {
        let conn = self.get_connection()?;
        let updated = conn.execute(
            "UPDATE transcription_history SET transcription_text = ?1 WHERE id = ?2",
            params![text, id],
        )?;

        if updated == 0 {
            return Err(anyhow!("History entry {} not found", id));
        }

        let entry = conn.query_row(
            &format!("{ENTRY_SELECT} WHERE h.id = ?1"),
            params![id],
            Self::map_history_entry,
        )?;

        if let Err(e) = (HistoryUpdatePayload::Updated {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(entry)
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
            "UPDATE transcription_history SET transcription_text = ?1 WHERE id = ?2",
            params![&update.transcription_text, id],
        )?;

        if updated == 0 {
            return Err(anyhow!("History entry {} not found", id));
        }

        // The existing statistics row is rewritten rather than joined by a
        // second one: a retry replaces the transcript, it does not mean the
        // user dictated twice. Counting both would inflate the word totals.
        tx.execute(
            "UPDATE usage_stats
             SET word_count = ?1,
                 processing_ms = ?2,
                 model_used = ?3,
                 language = ?4
             WHERE history_id = ?5",
            params![
                update.word_count,
                update.processing_ms,
                &update.model_used,
                &update.language,
                id,
            ],
        )?;

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

    /// Append one more refinement pass to an entry that already exists.
    ///
    /// Unlike [`Self::update_transcription`], nothing about the dictation
    /// changes: the transcript, the audio and every metric describe what was
    /// spoken, and running the text past a language model a second time does
    /// not alter any of that. Only `post_process_runs` grows — which is exactly
    /// what it was split into its own table for.
    ///
    /// The statistics row is deliberately untouched. It summarises the *first*
    /// attempt; letting a retry overwrite it would erase the failure from the
    /// numbers that exist to show how often refinement fails.
    pub fn append_post_process_run(&self, id: i64, run: NewPostProcessRun) -> Result<HistoryEntry> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        let exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM transcription_history WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Err(anyhow!("History entry {} not found", id));
        }

        Self::insert_post_process_run(&tx, id, Utc::now().timestamp(), run)?;
        tx.commit()?;

        let entry = conn.query_row(
            &format!("{ENTRY_SELECT} WHERE h.id = ?1"),
            params![id],
            Self::map_history_entry,
        )?;

        debug!("Appended a refinement run to history entry {}", id);

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

            // Delete WAV file, if this entry has one at all.
            let Some(file_path) = recording_path(&self.recordings_dir, file_name) else {
                continue;
            };
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

    /// Everything the Insights view shows, in one query pass.
    ///
    /// Assembled here rather than handing raw rows to the frontend: these are
    /// aggregates over a table that grows for years, and shipping thousands of
    /// rows across the bridge to sum them in JavaScript would be wasteful and
    /// would put the arithmetic somewhere it cannot be tested.
    pub fn usage_summary(&self, typing_wpm: f64) -> Result<UsageSummary> {
        let conn = self.get_connection()?;
        Self::usage_summary_with_conn(&conn, typing_wpm)
    }

    /// The arithmetic, separated from where the connection comes from — the
    /// tests can then check the aggregates against a database they built,
    /// rather than restating the same SQL and proving nothing.
    fn usage_summary_with_conn(conn: &Connection, typing_wpm: f64) -> Result<UsageSummary> {
        // Every aggregate below is about dictating, so each one restricts itself
        // to dictations. Rewrites are counted separately at the end — mixing
        // them in would make a three-word instruction weigh as much as a
        // paragraph in words per day, speaking rate and time saved.
        let (dictations, words, duration_ms, processing_ms): (i64, i64, i64, i64) = conn
            .query_row(
                "SELECT
                    COUNT(*),
                    COALESCE(SUM(word_count), 0),
                    COALESCE(SUM(duration_ms), 0),
                    COALESCE(SUM(processing_ms), 0)
                 FROM usage_stats
                 WHERE kind = 'dictation'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;

        let refined: i64 = conn.query_row(
            "SELECT COUNT(*) FROM usage_stats
             WHERE kind = 'dictation' AND post_process_succeeded = 1",
            [],
            |row| row.get(0),
        )?;

        let refinement_failures: i64 = conn.query_row(
            "SELECT COUNT(*) FROM usage_stats
             WHERE kind = 'dictation' AND post_process_succeeded = 0",
            [],
            |row| row.get(0),
        )?;

        let (rewrites, spoken_commands): (i64, i64) = conn.query_row(
            "SELECT
                COUNT(*),
                COALESCE(SUM(kind = 'command'), 0)
             FROM usage_stats
             WHERE kind IN ('command', 'rewrite')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        // Local time, not UTC: "which day did I dictate this" is a question
        // about the user's calendar, and a dictation at 01:00 belongs to the
        // night it happened in, not to the previous UTC day.
        let mut stmt = conn.prepare(
            "SELECT date(timestamp, 'unixepoch', 'localtime') AS day,
                    COALESCE(SUM(word_count), 0)
             FROM usage_stats
             WHERE kind = 'dictation'
             GROUP BY day
             ORDER BY day DESC
             LIMIT 30",
        )?;
        let mut per_day: Vec<DailyWords> = stmt
            .query_map([], |row| {
                Ok(DailyWords {
                    day: row.get(0)?,
                    words: row.get(1)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        per_day.reverse(); // oldest first, so a chart reads left to right

        let mut stmt = conn.prepare(
            "SELECT CAST(strftime('%H', timestamp, 'unixepoch', 'localtime') AS INTEGER),
                    COUNT(*)
             FROM usage_stats
             WHERE kind = 'dictation'
             GROUP BY 1
             ORDER BY 2 DESC, 1 ASC
             LIMIT 1",
        )?;
        let busiest_hour: Option<u32> = stmt
            .query_row([], |row| row.get::<_, i64>(0))
            .optional()?
            .map(|hour| hour as u32);

        let mut stmt = conn.prepare(
            "SELECT model_used, COUNT(*), COALESCE(AVG(processing_ms), 0)
             FROM usage_stats
             WHERE kind = 'dictation' AND model_used IS NOT NULL AND model_used != ''
             GROUP BY model_used
             ORDER BY 2 DESC",
        )?;
        let models: Vec<ModelUsage> = stmt
            .query_map([], |row| {
                Ok(ModelUsage {
                    model: row.get(0)?,
                    dictations: row.get(1)?,
                    average_processing_ms: row.get::<_, f64>(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Time saved: how long typing the same words would have taken, minus
        // the time actually spent speaking. Deliberately ignores the seconds
        // the model needed — the user is not waiting on those, they are already
        // reading or moving on.
        let typing_ms = if typing_wpm > 0.0 {
            (words as f64 / typing_wpm) * 60_000.0
        } else {
            0.0
        };
        let saved_ms = (typing_ms - duration_ms as f64).max(0.0) as i64;

        Ok(UsageSummary {
            dictations,
            words,
            duration_ms,
            processing_ms,
            refined,
            refinement_failures,
            saved_ms,
            busiest_hour,
            per_day,
            models,
            rewrites,
            spoken_commands,
        })
    }

    /// Every recorded row, for export. Numbers only — see the `usage_stats`
    /// migration for why there is no text in here.
    pub fn usage_rows(&self) -> Result<Vec<UsageRow>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT timestamp, kind, duration_ms, word_count, processing_ms, model_used,
                    language, post_process_requested, post_process_provider,
                    post_process_model, post_process_ms, post_process_succeeded
             FROM usage_stats
             ORDER BY timestamp ASC",
        )?;

        // Unfiltered: an export is meant to be complete, and the `kind` column
        // lets whoever reads it filter for themselves.
        let rows = stmt
            .query_map([], |row| {
                Ok(UsageRow {
                    timestamp: row.get(0)?,
                    kind: EntryKind::from_str(&row.get::<_, String>(1)?),
                    duration_ms: row.get(2)?,
                    word_count: row.get(3)?,
                    processing_ms: row.get(4)?,
                    model_used: row.get(5)?,
                    language: row.get(6)?,
                    post_process_requested: row.get(7)?,
                    post_process_provider: row.get(8)?,
                    post_process_model: row.get(9)?,
                    post_process_ms: row.get(10)?,
                    post_process_succeeded: row.get(11)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// Delete all statistics, leaving the history untouched.
    ///
    /// The other half of Northstar §6.3: the two must be erasable
    /// independently. Nothing here cascades — `usage_stats` has no foreign key
    /// precisely so the two lifetimes stay separate.
    pub fn clear_usage_stats(&self) -> Result<usize> {
        let conn = self.get_connection()?;
        let deleted = conn.execute("DELETE FROM usage_stats", [])?;
        info!("Cleared {} usage statistics rows", deleted);
        Ok(deleted)
    }

    /// Average words per dictation, from the recorded statistics.
    ///
    /// This is what turns a catalogue price into an answer: "$3 per million
    /// tokens" says nothing useful, "roughly this much per dictation" does —
    /// and only the user's own history knows how long their dictations are.
    ///
    /// `None` until enough has been dictated to mean anything; a single
    /// three-word test would otherwise set the yardstick.
    pub fn average_word_count(&self) -> Result<Option<f64>> {
        const MINIMUM_SAMPLE: i64 = 5;

        let conn = self.get_connection()?;
        let (count, total): (i64, Option<i64>) = conn.query_row(
            "SELECT COUNT(*), SUM(word_count) FROM usage_stats
             WHERE kind = 'dictation' AND word_count IS NOT NULL AND word_count > 0",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        if count < MINIMUM_SAMPLE {
            return Ok(None);
        }

        Ok(total.map(|total| total as f64 / count as f64))
    }

    /// Turn user input into a safe fts5 query.
    ///
    /// fts5 `MATCH` takes a query language of its own: bare `AND`, `OR`, `-`,
    /// `*` or an unbalanced quote are syntax, and a stray one makes the whole
    /// query fail rather than find nothing. Someone typing into a search box
    /// means none of that, so every token is quoted (with `"` doubled to escape
    /// it) and a `*` appended, which is what makes the search feel live while
    /// still typing a word.
    fn to_fts_query(input: &str) -> Option<String> {
        let query = input
            .split_whitespace()
            .map(|token| format!("\"{}\"*", token.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" ");

        (!query.is_empty()).then_some(query)
    }

    /// Search transcripts, optionally limited to saved entries.
    ///
    /// An empty query is not an error — it means "no text filter", so the
    /// favourites toggle works on its own.
    pub async fn search_history_entries(
        &self,
        query: Option<String>,
        only_saved: bool,
        limit: Option<usize>,
    ) -> Result<Vec<HistoryEntry>> {
        let conn = self.get_connection()?;
        let limit = limit.unwrap_or(100).min(500) as i64;
        let fts_query = query.as_deref().and_then(Self::to_fts_query);

        let saved_clause = if only_saved { " AND h.saved = 1" } else { "" };

        let (sql, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match fts_query {
            Some(fts) => (
                // Ranked by fts5's own relevance, not by date: when searching,
                // the best match matters more than the newest entry.
                format!(
                    "{ENTRY_SELECT}
                     JOIN transcription_search ON transcription_search.rowid = h.id
                     WHERE transcription_search MATCH ?1{saved_clause}
                     ORDER BY rank
                     LIMIT ?2"
                ),
                vec![Box::new(fts), Box::new(limit)],
            ),
            None => (
                format!("{ENTRY_SELECT} WHERE 1 = 1{saved_clause} ORDER BY h.id DESC LIMIT ?1"),
                vec![Box::new(limit)],
            ),
        };

        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter()),
                Self::map_history_entry,
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
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

    /// Run a search the way the manager does, but against a bare connection.
    fn search_ids(conn: &Connection, query: &str) -> Vec<i64> {
        let fts = HistoryManager::to_fts_query(query).expect("non-empty query");
        let mut stmt = conn
            .prepare(
                "SELECT rowid FROM transcription_search
                 WHERE transcription_search MATCH ?1 ORDER BY rank",
            )
            .expect("prepare search");

        stmt.query_map([&fts], |row| row.get::<_, i64>(0))
            .expect("run search")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("collect hits")
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

        // Held on to: every further insert moves `last_insert_rowid` on.
        let history_id = conn.last_insert_rowid();

        // The manager writes this row alongside every entry; the tests have to
        // do the same or the statistics assertions would be vacuous.
        conn.execute(
            "INSERT INTO usage_stats (history_id, timestamp, duration_ms, word_count, model_used)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                history_id,
                timestamp,
                4200,
                text.split_whitespace().count() as i64,
                "whisper-large-v3-turbo",
            ],
        )
        .expect("insert usage stats");

        if let Some(output) = post_processed {
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

    /// An entry that rewrote existing text. `kind` is what the statistics key
    /// off, and a rewrite has no recording — hence the empty file name.
    fn insert_rewrite(conn: &Connection, timestamp: i64, kind: EntryKind, text: &str) {
        conn.execute(
            "INSERT INTO transcription_history (
                file_name, timestamp, saved, title, transcription_text,
                post_process_requested, kind
            ) VALUES ('', ?1, 0, ?2, ?3, 1, ?4)",
            params![
                timestamp,
                format!("Recording {}", timestamp),
                text,
                kind.as_str(),
            ],
        )
        .expect("insert rewrite entry");

        conn.execute(
            "INSERT INTO usage_stats (history_id, timestamp, kind, duration_ms, word_count, model_used)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                conn.last_insert_rowid(),
                timestamp,
                kind.as_str(),
                1500,
                text.split_whitespace().count() as i64,
                "whisper-large-v3-turbo",
            ],
        )
        .expect("insert usage stats");
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

    /// A successful retry after a failure: the newer run wins for display, and
    /// the failed one stays on record. Losing it would erase the evidence that
    /// the refinement model is unreliable — the very thing the statistics count.
    #[test]
    fn a_retry_is_an_additional_run_not_a_replacement() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "raw", None);

        let failed = NewPostProcessRun {
            provider_id: "ollama".to_string(),
            model: Some("llama3.2".to_string()),
            prompt_id: Some("preset_tidy".to_string()),
            prompt_text: Some("tidy this".to_string()),
            input_text: "raw".to_string(),
            output_text: None,
            duration_ms: Some(1200),
            succeeded: false,
            error: Some("connection refused".to_string()),
        };
        HistoryManager::insert_post_process_run(&conn, 1, 100, failed.clone())
            .expect("insert failed run");

        HistoryManager::insert_post_process_run(
            &conn,
            1,
            200,
            NewPostProcessRun {
                output_text: Some("tidied".to_string()),
                succeeded: true,
                error: None,
                ..failed
            },
        )
        .expect("insert retry");

        let entry = HistoryManager::get_latest_entry_with_conn(&conn)
            .expect("fetch entry")
            .expect("entry exists");

        assert_eq!(entry.display_text(), "tidied");
        assert!(entry.last_post_process.as_ref().unwrap().succeeded);

        let runs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM post_process_runs WHERE history_id = 1",
                [],
                |row| row.get(0),
            )
            .expect("count runs");
        assert_eq!(runs, 2, "the failed attempt must survive the retry");
    }

    /// A retry reads its input from the recorded run, not from the transcript.
    /// For a Command Mode entry those are different things — the transcript is
    /// the spoken instruction — and confusing them would rewrite the
    /// instruction instead of the text it applied to.
    #[test]
    fn a_command_entry_keeps_instruction_and_material_apart() {
        let conn = setup_conn();
        insert_rewrite(&conn, 100, EntryKind::Command, "mach das kuerzer");

        HistoryManager::insert_post_process_run(
            &conn,
            1,
            100,
            NewPostProcessRun {
                provider_id: "ollama".to_string(),
                model: Some("llama3.2".to_string()),
                prompt_id: None,
                prompt_text: None,
                input_text: "Ein langer Absatz, der gekuerzt werden soll.".to_string(),
                output_text: None,
                duration_ms: Some(900),
                succeeded: false,
                error: Some("timeout".to_string()),
            },
        )
        .expect("insert failed run");

        let entry = HistoryManager::get_latest_entry_with_conn(&conn)
            .expect("fetch entry")
            .expect("entry exists");

        assert_eq!(entry.kind, EntryKind::Command);
        assert_eq!(entry.transcription_text, "mach das kuerzer");
        assert_eq!(
            entry.last_post_process.as_ref().unwrap().input_text,
            "Ein langer Absatz, der gekuerzt werden soll."
        );
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

    /// The whole point of the separate table: `cleanup_by_count` deletes
    /// transcripts, and the statistics must not go with them. With the default
    /// limit of five entries, anything else would cap the statistics at five
    /// dictations forever.
    #[test]
    fn statistics_survive_the_history_being_cleaned_up() {
        let conn = setup_conn();
        for timestamp in 1..=10 {
            insert_entry(&conn, timestamp, "gesprochener text", None);
        }

        // Simulate the cleanup keeping only the newest three entries.
        conn.execute("DELETE FROM transcription_history WHERE id <= 7", [])
            .expect("cleanup");

        let entries: i64 = conn
            .query_row("SELECT COUNT(*) FROM transcription_history", [], |r| {
                r.get(0)
            })
            .expect("count entries");
        let stats: i64 = conn
            .query_row("SELECT COUNT(*) FROM usage_stats", [], |r| r.get(0))
            .expect("count stats");

        assert_eq!(entries, 3);
        assert_eq!(stats, 10, "statistics must outlive the transcripts");
    }

    /// The statistics table must never be able to reveal *what* was dictated —
    /// that is what makes it safe to keep indefinitely.
    #[test]
    fn statistics_hold_no_transcript_text() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "streng geheimer inhalt", Some("veredelt"));

        let columns: Vec<String> = conn
            .prepare("SELECT * FROM usage_stats")
            .expect("prepare")
            .column_names()
            .into_iter()
            .map(str::to_string)
            .collect();

        for forbidden in [
            "transcription_text",
            "output_text",
            "input_text",
            "prompt_text",
            "file_name",
            "title",
        ] {
            assert!(
                !columns.contains(&forbidden.to_string()),
                "usage_stats must not carry {forbidden}"
            );
        }
    }

    #[test]
    fn full_text_search_finds_entries() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "Termin beim Zahnarzt vereinbaren", None);
        insert_entry(&conn, 200, "Einkaufsliste: Brot und Milch", None);

        let hits = search_ids(&conn, "zahnarzt");
        assert_eq!(hits.len(), 1);

        // Prefix matching, so the search reacts while still typing.
        assert_eq!(search_ids(&conn, "einkauf").len(), 1);
        assert_eq!(search_ids(&conn, "milch brot").len(), 1);
        assert!(search_ids(&conn, "urlaub").is_empty());
    }

    /// fts5 has a query language of its own. Input from a search box is not
    /// written in it, so operators must be treated as literal text rather than
    /// making the whole query fail.
    #[test]
    fn search_input_is_not_treated_as_query_syntax() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "Ergebnis war gut", None);

        for input in ["AND", "OR", "-", "\"", "*", "NEAR(", "gut OR"] {
            let query = HistoryManager::to_fts_query(input);
            if let Some(query) = query {
                let result = conn
                    .prepare("SELECT rowid FROM transcription_search WHERE transcription_search MATCH ?1")
                    .expect("prepare")
                    .query_map([&query], |row| row.get::<_, i64>(0))
                    .map(|rows| rows.count());
                assert!(result.is_ok(), "{input:?} must not break the query");
            }
        }
    }

    /// An external-content fts5 index does not notice deletions by itself. A
    /// missing delete trigger leaves a phantom that keeps matching searches
    /// after the entry is long gone.
    #[test]
    fn deleting_an_entry_removes_it_from_the_search_index() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "Zahnarzttermin", None);
        assert_eq!(search_ids(&conn, "zahnarzttermin").len(), 1);

        conn.execute("DELETE FROM transcription_history WHERE id = 1", [])
            .expect("delete entry");

        assert!(search_ids(&conn, "zahnarzttermin").is_empty());
    }

    /// Statistics must be erasable without taking the transcripts with them —
    /// the other half of what the separate table exists for (§6.3).
    #[test]
    fn clearing_statistics_leaves_the_history_alone() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "erstes diktat", None);
        insert_entry(&conn, 200, "zweites diktat", None);

        conn.execute("DELETE FROM usage_stats", []).expect("clear");

        let entries: i64 = conn
            .query_row("SELECT COUNT(*) FROM transcription_history", [], |r| {
                r.get(0)
            })
            .expect("count entries");

        assert_eq!(entries, 2, "history must survive clearing the statistics");
    }

    /// Days are grouped by the user's calendar, not by UTC: a dictation just
    /// after midnight belongs to the night it happened in.
    #[test]
    fn daily_totals_group_by_local_calendar_day() {
        let conn = setup_conn();
        insert_entry(&conn, 1_754_000_000, "drei kurze woerter", None);
        insert_entry(&conn, 1_754_003_600, "eine stunde spaeter gesprochen", None);

        let mut stmt = conn
            .prepare(
                "SELECT date(timestamp, 'unixepoch', 'localtime'), SUM(word_count)
                 FROM usage_stats GROUP BY 1",
            )
            .expect("prepare");

        let days: Vec<(String, i64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("collect");

        assert!(!days.is_empty());
        for (day, words) in &days {
            assert_eq!(day.len(), 10, "expected YYYY-MM-DD, got {day}");
            assert!(*words > 0);
        }
    }

    /// The whole point of the `kind` column: an instruction like "kürzer" is
    /// three words spoken over someone else's paragraph. Counted as a dictation
    /// it would drag down words per day, speaking rate and time saved at once.
    #[test]
    fn rewrites_do_not_count_as_dictations() {
        let conn = setup_conn();
        insert_entry(
            &conn,
            100,
            "ein ordentlich langes diktat mit sechs woertern",
            None,
        );
        insert_rewrite(&conn, 200, EntryKind::Command, "kuerzer");
        insert_rewrite(&conn, 300, EntryKind::Rewrite, "irgendein markierter text");

        let summary = HistoryManager::usage_summary_with_conn(&conn, 40.0).expect("summary");

        assert_eq!(summary.dictations, 1);
        assert_eq!(summary.words, 7, "only the dictated words are counted");
        assert_eq!(
            summary.duration_ms, 4200,
            "only the dictation's speaking time"
        );
        assert_eq!(summary.rewrites, 2);
        assert_eq!(summary.spoken_commands, 1);
    }

    /// Model statistics describe transcription. A rewrite records the same model
    /// name because it is the one loaded, but nothing was transcribed for it.
    #[test]
    fn model_usage_ignores_rewrites() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "erstes diktat", None);
        insert_rewrite(&conn, 200, EntryKind::Rewrite, "markierter text");

        let summary = HistoryManager::usage_summary_with_conn(&conn, 40.0).expect("summary");

        let model = summary.models.first().expect("one model");
        assert_eq!(model.dictations, 1);
    }

    /// The export is the one place that stays unfiltered — with the kind in the
    /// row, whoever reads it can separate the two for themselves.
    #[test]
    fn export_keeps_every_row_and_says_what_it_is() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "ein diktat", None);
        insert_rewrite(&conn, 200, EntryKind::Command, "foermlicher");

        let mut stmt = conn
            .prepare("SELECT kind FROM usage_stats ORDER BY timestamp")
            .expect("prepare");
        let kinds: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("collect");

        assert_eq!(kinds, vec!["dictation", "command"]);
    }

    /// Everything recorded before the column existed was dictated, so the
    /// default has to hold retroactively — otherwise the migration would empty
    /// out every statistic that came before it.
    #[test]
    fn entries_from_before_the_column_count_as_dictations() {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");

        // Migrate to the schema shipped in 0.16.0, write a row the way that
        // version would have, then migrate the rest of the way.
        let up_to_fts = MIGRATIONS.len() - 1;
        Migrations::new(MIGRATIONS[..up_to_fts].to_vec())
            .to_latest(&mut conn)
            .expect("run migrations up to 0.16.0");
        conn.execute(
            "INSERT INTO transcription_history (
                file_name, timestamp, saved, title, transcription_text,
                post_process_requested
            ) VALUES ('a.wav', 100, 0, 'Recording', 'ein altes diktat', 0)",
            [],
        )
        .expect("insert pre-migration entry");
        conn.execute(
            "INSERT INTO usage_stats (history_id, timestamp, word_count, duration_ms)
             VALUES (1, 100, 3, 4200)",
            [],
        )
        .expect("insert pre-migration stats");

        Migrations::new(MIGRATIONS.to_vec())
            .to_latest(&mut conn)
            .expect("run remaining migrations");

        let summary = HistoryManager::usage_summary_with_conn(&conn, 40.0).expect("summary");
        assert_eq!(summary.dictations, 1, "old rows must still count");
        assert_eq!(summary.words, 3);

        let entry = HistoryManager::get_latest_entry_with_conn(&conn)
            .expect("fetch entry")
            .expect("entry exists");
        assert_eq!(entry.kind, EntryKind::Dictation);
    }

    /// The trap this guards against: `join("")` is the directory itself, and it
    /// exists, so an `exists()` check would wave it straight through to
    /// `remove_file` — on the folder holding every recording.
    #[test]
    fn an_entry_without_a_recording_has_no_file_to_delete() {
        let dir = std::path::Path::new("/tmp/murmel-recordings");

        assert_eq!(recording_path(dir, ""), None);
        assert_eq!(
            recording_path(dir, "murmel-1.wav"),
            Some(dir.join("murmel-1.wav"))
        );
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
