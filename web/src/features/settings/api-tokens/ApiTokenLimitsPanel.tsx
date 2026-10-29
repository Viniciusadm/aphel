import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { useOrganizationConfiguration, useSaveConfiguration } from '../general/api'
import { NumericSettingsPanel } from '../general/NumericSettingsPanel'
import { SETTINGS_MANAGE } from '../lib/permissions'

export function ApiTokenLimitsPanel() {
  const { t } = useTranslation('settings')
  const { hasPermission } = useSession()
  const canManage = hasPermission(SETTINGS_MANAGE)
  const configuration = useOrganizationConfiguration(canManage)
  const save = useSaveConfiguration()
  if (!canManage || !configuration.data) return null
  const limits = configuration.data.api_tokens
  return (
    <NumericSettingsPanel
      key={JSON.stringify(limits)}
      id="api-token-limits"
      title={t('configuration.apiTokens.title')}
      description={t('configuration.apiTokens.description')}
      serverPrefix="api_tokens"
      numbers={[
        { key: 'limit', label: t('configuration.apiTokens.limit'), min: 1, max: 100000, value: limits.limit },
        {
          key: 'window_secs',
          label: t('configuration.apiTokens.window'),
          min: 1,
          max: 3600,
          value: limits.window_secs,
        },
      ]}
      onSave={async (numbers) => {
        await save({
          api_tokens: {
            limit: numbers.limit ?? limits.limit,
            window_secs: numbers.window_secs ?? limits.window_secs,
          },
        })
      }}
    />
  )
}
