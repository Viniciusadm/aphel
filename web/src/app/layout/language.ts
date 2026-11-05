import i18n, { DEFAULT_LANGUAGE } from '@/i18n'
import { availableLanguages, switchLanguage } from '@/kernel/modules/i18n'
import { readStorage, writeStorage } from '@/lib/storage'

export type AppLanguage = string

const LANGUAGE_STORAGE_KEY = 'aphel.language'

export function normalizeLanguage(value: string | null | undefined): AppLanguage | null {
  if (!value) return null
  const lower = value.toLowerCase()
  const languages = availableLanguages()
  const exact = languages.find((language) => language.toLowerCase() === lower)
  if (exact) return exact
  const primary = lower.split('-')[0]
  return languages.find((language) => language.toLowerCase().split('-')[0] === primary) ?? null
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
