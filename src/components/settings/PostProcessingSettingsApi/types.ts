export type ModelOption = {
  value: string;
  label: string;
};

/**
 * The two providers Murmel offers, kept in step with `settings.rs`.
 *
 * Local means the dictation never leaves the machine; remote means it does.
 * That distinction runs through the whole refinement UI, so the ids are named
 * here once rather than typed out at each site.
 */
export const OLLAMA_PROVIDER_ID = "ollama";
export const OPENROUTER_PROVIDER_ID = "openrouter";
