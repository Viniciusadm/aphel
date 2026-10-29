import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'

export const DEFAULT_LANGUAGE = 'pt-BR'
export const SUPPORTED_LANGUAGES = ['pt-BR', 'en'] as const

void i18n.use(initReactI18next).init({
  resources: {},
  lng: DEFAULT_LANGUAGE,
  fallbackLng: DEFAULT_LANGUAGE,
  supportedLngs: SUPPORTED_LANGUAGES,
  defaultNS: 'common',
  interpolation: { escapeValue: false },
  returnNull: false,
  initAsync: false,
})

export default i18n
