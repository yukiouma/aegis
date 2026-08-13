import type { TranslationKey } from "@aegis/ui/i18n";

export type LogLevel = "info" | "success" | "error";

/**
 * A single line in the splash log. It stores the translation *key* and its
 * params rather than translated text, so entries logged before a language
 * switch re-render in the new language.
 */
export interface LogEntry {
  id: number;
  level: LogLevel;
  key: TranslationKey;
  params?: Record<string, string>;
}

export type PushLog = (
  level: LogLevel,
  key: TranslationKey,
  params?: Record<string, string>,
) => void;
