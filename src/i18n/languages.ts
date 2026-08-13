/**
 * Language metadata for supported locales.
 *
 * Murmel ships German and English only. The fork inherited 24 languages, but a
 * single maintainer cannot check a translation they do not speak — and every
 * new string would otherwise cost 24 entries, 22 of them machine-made or filled
 * with English. See Murmel_Northstar.md §4.2.
 *
 * To add a language:
 * 1. Create a new folder: src/i18n/locales/{code}/translation.json
 * 2. Add an entry here with the language code, English name, and native name
 * 3. Optionally add a priority (lower = higher in dropdown, no priority = alphabetical at end)
 * 4. For RTL languages, add direction: 'rtl'
 */
export const LANGUAGE_METADATA: Record<
  string,
  {
    name: string;
    nativeName: string;
    priority?: number;
    direction?: "ltr" | "rtl";
  }
> = {
  de: { name: "German", nativeName: "Deutsch", priority: 1 },
  en: { name: "English", nativeName: "English", priority: 2 },
};
