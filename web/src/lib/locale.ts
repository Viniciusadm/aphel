import i18n, { DEFAULT_LANGUAGE } from '@/i18n'

const INTL_LOCALES: Record<string, string> = {
  en: 'en-US',
  'pt-BR': 'pt-BR',
}

export function intlLocale(language: string | undefined): string {
  const resolved = language || DEFAULT_LANGUAGE
  return INTL_LOCALES[resolved] ?? resolved
}

export function activeLocale(): string {
  return intlLocale(i18n.resolvedLanguage ?? i18n.language)
}
