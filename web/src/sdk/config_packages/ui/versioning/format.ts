import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

export function useDateFormatter() {
  const { i18n } = useTranslation()
  return useMemo(
    () => new Intl.DateTimeFormat(i18n.language, { dateStyle: 'short', timeStyle: 'short' }),
    [i18n.language],
  )
}
