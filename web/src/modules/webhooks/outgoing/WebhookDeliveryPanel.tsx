import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { NumericSettingsPanel, SETTINGS_MANAGE, useOrganizationConfiguration, useSaveConfiguration } from '@/sdk'
export function WebhookDeliveryPanel() {
  const { t } = useTranslation('settings')
  const { hasPermission } = useSession()
  const canManage = hasPermission(SETTINGS_MANAGE)
  const configuration = useOrganizationConfiguration(canManage)
  const save = useSaveConfiguration()
  if (!canManage || !configuration.data) return null
  const delivery = configuration.data.webhooks
  return (
    <NumericSettingsPanel
      key={JSON.stringify(delivery)}
      id="webhook-delivery"
      title={t('configuration.webhooks.title')}
      description={t('configuration.webhooks.description')}
      serverPrefix="webhooks"
      numbers={[
        { key: 'timeout_secs', label: t('configuration.webhooks.timeout'), min: 1, max: 60, value: delivery.timeout_secs },
        {
          key: 'disable_after_failures',
          label: t('configuration.webhooks.disableAfterFailures'),
          min: 1,
          max: 1000,
          value: delivery.disable_after_failures,
        },
        {
          key: 'retention_days',
          label: t('configuration.webhooks.retentionDays'),
          min: 1,
          max: 365,
          value: delivery.retention_days,
        },
      ]}
      toggles={[{ key: 'require_https', label: t('configuration.webhooks.requireHttps'), value: delivery.require_https }]}
      onSave={async (numbers, toggles) => {
        await save({
          webhooks: {
            timeout_secs: numbers.timeout_secs ?? delivery.timeout_secs,
            disable_after_failures: numbers.disable_after_failures ?? delivery.disable_after_failures,
            retention_days: numbers.retention_days ?? delivery.retention_days,
            require_https: toggles.require_https ?? delivery.require_https,
          },
        })
      }}
    />
  )
}
