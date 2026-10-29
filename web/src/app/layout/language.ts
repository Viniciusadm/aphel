import i18n, { DEFAULT_LANGUAGE, SUPPORTED_LANGUAGES } from '@/i18n'
import { switchLanguage } from '@/kernel/modules/i18n'
import { readStorage, writeStorage } from '@/lib/storage'

export type AppLanguage = (typeof SUPPORTED_LANGUAGES)[number]

const LANGUAGE_STORAGE_KEY = 'aphel.language'

export function normalizeLanguage(value: string | null | undefined): AppLanguage | null {
  if (!value) return null
  const lower = value.toLowerCase()
  if (lower === 'en' || lower.startsWith('en-')) return 'en'
  if (lower === 'pt' || lower.startsWith('pt-')) return 'pt-BR'
  return null
}

export function applyLanguage(language: AppLanguage): void {
  writeStorage(LANGUAGE_STORAGE_KEY, language)
  if (typeof document !== 'undefined') document.documentElement.lang = language
  if (i18n.language !== language) void switchLanguage(language)
}

export function applyStoredLanguage(): void {
  const stored = normalizeLanguage(readStorage(LANGUAGE_STORAGE_KEY))
  if (stored && stored !== DEFAULT_LANGUAGE) applyLanguage(stored)
}
