/**
 * Jumping between settings sections from within a section.
 *
 * A deliberately small mechanism: the alternative would be threading a
 * navigation callback through every component that might ever want to link
 * somewhere, and the only thing that needs to travel is a section name.
 *
 * Used where a screen shows a setting it does not own — the model list naming
 * the active provider, for instance, which is configured one section over.
 */
const EVENT = "murmel:navigate";

export function navigateToSection(section: string): void {
  window.dispatchEvent(new CustomEvent(EVENT, { detail: section }));
}

/** Returns an unsubscribe function, for use in an effect. */
export function onNavigateToSection(
  handler: (section: string) => void,
): () => void {
  const listener = (event: Event) => {
    const section = (event as CustomEvent<string>).detail;
    if (typeof section === "string") handler(section);
  };

  window.addEventListener(EVENT, listener);
  return () => window.removeEventListener(EVENT, listener);
}
