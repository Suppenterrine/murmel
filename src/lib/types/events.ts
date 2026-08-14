export interface ModelStateEvent {
  event_type: string;
  model_id?: string;
  model_name?: string;
  error?: string;
}

/**
 * A failure worth telling the user about — see `src-tauri/src/problem.rs`.
 *
 * Carries a translation key rather than a sentence: the wording belongs in the
 * locale files with the rest of the UI, and a formatted string from Rust would
 * be English for everyone or duplicate the translations there.
 */
export interface ProblemReport {
  /** Key suffix under `problem.*`. */
  key: string;
  /** The technical reason. Also in murmel.log. */
  detail: string | null;
  /** Whether the history holds a recording that can be retried. */
  recoverable: boolean;
}
