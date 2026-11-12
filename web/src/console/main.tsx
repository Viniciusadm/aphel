import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { onApiError } from '@/api'
import i18n, { DEFAULT_LANGUAGE } from '@/i18n'
import { KERNEL_TRANSLATIONS } from '@/modules/kernel/translations'
import { ConsoleApp } from './ConsoleApp'
import { consoleRouter } from './router'
import { SESSION_REQUIRED_CODE, SIGN_IN_PATH, isPublicPath } from './paths'

const NAMESPACES = ['common', 'records-detail', 'records-form', 'modules', 'console']

function preferredLanguage(): string {
  return navigator.language.toLowerCase().startsWith('en') ? 'en' : DEFAULT_LANGUAGE
}

async function loadTranslations(language: string): Promise<void> {
  for (const namespace of NAMESPACES) {
    for (const locale of new Set([language, DEFAULT_LANGUAGE])) {
      const source = KERNEL_TRANSLATIONS[namespace]?.[locale]
      if (!source) continue
      const loaded = typeof source === 'function' ? await source() : source
      const resources = 'default' in loaded ? (loaded.default as Record<string, unknown>) : loaded
      i18n.addResourceBundle(locale, namespace, resources, true, true)
    }
  }
  await i18n.changeLanguage(language)
}

export async function startConsole(): Promise<void> {
  const language = preferredLanguage()
  await loadTranslations(language)
  document.documentElement.lang = language
  document.title = i18n.t('brand', { ns: 'console' })
  onApiError((error) => {
    if (error.code !== SESSION_REQUIRED_CODE || isPublicPath(window.location.pathname)) return
    void consoleRouter.navigate(SIGN_IN_PATH)
  })
  const rootElement = document.getElementById('root')
  if (!rootElement) throw new Error('root element not found')
  createRoot(rootElement).render(
    <StrictMode>
      <ConsoleApp />
    </StrictMode>,
  )
}
