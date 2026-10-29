import type { ParseKeys } from 'i18next'
import { useTranslation } from 'react-i18next'

export type SchedulingKey = ParseKeys<'scheduling'>

export function useSchedulingText() {
  const { t } = useTranslation('scheduling')
  return (key: string) => t(key as SchedulingKey)
}
