import { createI18n } from "vue-i18n";
import en from "./locales/en.json";
import ptBR from "./locales/pt-BR.json";

export const LOCALES = ["en", "pt-BR"] as const;
export type Locale = (typeof LOCALES)[number];

const STORAGE_KEY = "sizewise.locale";

export const i18n = createI18n({
  legacy: false,
  locale: "en",
  fallbackLocale: "en",
  messages: { en, "pt-BR": ptBR },
});

/** Portuguese for any `pt-*` system language, English for everything else. */
export function toSupportedLocale(tag: string | null | undefined): Locale {
  return tag?.toLowerCase().startsWith("pt") ? "pt-BR" : "en";
}

function storedLocale(): Locale | null {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return LOCALES.find((l) => l === value) ?? null;
  } catch {
    return null;
  }
}

/** The user's saved choice wins; otherwise the Windows display language. */
export async function initLocale(systemLocale: () => Promise<string | null>): Promise<void> {
  let locale = storedLocale();
  if (!locale) {
    let system: string | null = null;
    try {
      system = await systemLocale();
    } catch {
      system = navigator.language;
    }
    locale = toSupportedLocale(system ?? navigator.language);
  }
  applyLocale(locale);
}

export function setLocale(locale: Locale): void {
  applyLocale(locale);
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    // Not persisted; the choice still applies for this session.
  }
}

function applyLocale(locale: Locale): void {
  i18n.global.locale.value = locale;
  document.documentElement.lang = locale;
}
