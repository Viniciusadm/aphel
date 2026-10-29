import { useEffect } from 'react'
import { useSession } from '@/app/session/session-context'
import { DEFAULT_LANGUAGE } from '@/i18n'
import { applyLanguage, normalizeLanguage } from './language'

export function LanguageSync() {
  const { me } = useSession()
  const language =
    normalizeLanguage(me.user.locale) ?? normalizeLanguage(me.organization?.locale) ?? DEFAULT_LANGUAGE

  useEffect(() => {
    applyLanguage(language)
  }, [language])

  return null
}
