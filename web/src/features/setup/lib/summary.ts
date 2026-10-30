import { currencyLabel } from '@/features/settings/organization/lib/preferences'
import type { SetupValues } from './form'

export function companySummary(values: SetupValues, language: string, localeLabel: string): string {
  return [values.organizationName.trim(), localeLabel, currencyLabel(values.currency, language), values.timezone.replaceAll('_', ' ')]
    .filter(Boolean)
    .join(' · ')
}
