import type { ParseKeys } from 'i18next'
import { useTranslation } from 'react-i18next'

export type LeadDistributionKey = ParseKeys<'assignment'>

export function useLeadDistributionText() {
  const { t } = useTranslation('assignment')
  return (key: string) => t(key as LeadDistributionKey)
}
