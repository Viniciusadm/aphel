import { useTranslation } from 'react-i18next'
import type { Integration, IntegrationField } from './api'

export function useIntegrationText() {
  const { t, i18n } = useTranslation('integration-card')
  const lookup = (key: string, fallback: string): string => {
    const fullKey = `integration-card:${key}`
    return i18n.exists(fullKey) ? String(t(key as never)) : fallback
  }
  return {
    name: (integration: Integration) => lookup(`providers.${integration.provider_key}.name`, integration.name),
    description: (integration: Integration) =>
      lookup(`providers.${integration.provider_key}.description`, integration.description),
    fieldLabel: (integration: Integration, field: IntegrationField) =>
      lookup(`providers.${integration.provider_key}.fields.${field.key}.label`, field.key),
    fieldHelp: (integration: Integration, field: IntegrationField) =>
      lookup(`providers.${integration.provider_key}.fields.${field.key}.help`, ''),
    optionLabel: (integration: Integration, field: IntegrationField, option: string) =>
      lookup(`providers.${integration.provider_key}.fields.${field.key}.options.${option}`, option),
  }
}
