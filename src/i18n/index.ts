import { emit, listen } from "@tauri-apps/api/event";
import { en, type Dictionary } from "./en";
import { ru } from "./ru";

export type { Dictionary };

export const locales = [
  { id: "en", native: "English", english: "English" },
  { id: "ru", native: "Русский", english: "Russian" },
] as const;

export type Locale = (typeof locales)[number]["id"];

export const dictionaries: Record<Locale, Dictionary> = { en, ru };

const STORAGE_KEY = "elin.locale";
const EVENT = "elin-locale";

export function isLocale(value: string | null | undefined): value is Locale {
  return Boolean(value && value in dictionaries);
}

export function detectLocale(): Locale {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isLocale(stored)) return stored;
  } catch {
    /* private mode / tests */
  }
  const nav = navigator.language.toLowerCase();
  const hit = locales.find((item) => nav === item.id || nav.startsWith(`${item.id}-`));
  return hit?.id ?? "en";
}

export function applyLocale(next: Locale) {
  if (!isLocale(next)) return;
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    /* ignore */
  }
  document.documentElement.lang = next;
  void emit(EVENT, next).catch(() => undefined);
}

export function subscribeLocale(onChange: (locale: Locale) => void): () => void {
  let unlisten: (() => void) | undefined;
  void listen<string>(EVENT, (event) => {
    if (isLocale(event.payload)) {
      document.documentElement.lang = event.payload;
      onChange(event.payload);
    }
  }).then((fn) => {
    unlisten = fn;
  });
  const onStorage = (event: StorageEvent) => {
    if (event.key === STORAGE_KEY && isLocale(event.newValue)) {
      document.documentElement.lang = event.newValue;
      onChange(event.newValue);
    }
  };
  window.addEventListener("storage", onStorage);
  return () => {
    unlisten?.();
    window.removeEventListener("storage", onStorage);
  };
}
