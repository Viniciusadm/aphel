import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'

export const DEFAULT_LANGUAGE = 'pt-BR'
export const SHIPPED_LANGUAGES = ['pt-BR', 'en'] as const

void i18n.use(initReactI18next).init({
  resources: {},
  lng: DEFAULT_LANGUAGE,
  fallbackLng: DEFAULT_LANGUAGE,
  defaultNS: 'common',
  interpolation: { escapeValue: false },
  returnNull: false,
  react: { bindI18nStore: 'added' },
  initAsync: false,
})

const brazilianCardinal = {
  select: (count: number) => (count === 1 ? 'one' : 'other'),
  resolvedOptions: () => ({ pluralCategories: ['one', 'other'] }),
}
const resolver = i18n.services.pluralResolver
const resolveRule = resolver.getRule.bind(resolver)
resolver.getRule = ((code: string, options?: { ordinal?: boolean }) =>
  code.startsWith('pt') && !options?.ordinal ? brazilianCardinal : resolveRule(code, options)) as typeof resolver.getRule

export default i18n
