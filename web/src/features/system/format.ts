import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

export function useDateTimeFormat() {
  const { i18n } = useTranslation()
  return useMemo(
    () => new Intl.DateTimeFormat(i18n.language, { dateStyle: 'short', timeStyle: 'medium' }),
    [i18n.language],
  )
}

export function useDurationFormat() {
  const { t } = useTranslation('system')
  return (seconds: number) => {
    if (seconds < 120) return t('duration.seconds', { count: seconds })
    if (seconds < 7_200) return t('duration.minutes', { count: Math.round(seconds / 60) })
    if (seconds < 172_800) return t('duration.hours', { count: Math.round(seconds / 3_600) })
    return t('duration.days', { count: Math.round(seconds / 86_400) })
  }
}
