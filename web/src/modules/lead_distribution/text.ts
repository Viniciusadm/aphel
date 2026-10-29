import type { ParseKeys } from 'i18next'
import { useTranslation } from 'react-i18next'

export type LeadDistributionKey = ParseKeys<'lead_distribution'>

export function useLeadDistributionText() {
  const { t } = useTranslation('lead_distribution')
  return (key: string) => t(key as LeadDistributionKey)
}
