import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess, PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Notice } from '@/components/ui/notice'
import { OptionCard } from '@/components/ui/option-card'
import { ErrorState } from '@/components/ui/error-state'
import { DEAL_AMOUNT_MODES, settingsApi, settingsKeys, useLineItemSettings, type AmountMode, type LineItemSettings } from './settings-api'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function LineItemSettingsPage() {
  const { t } = useTranslation('line_items')
  const query = useLineItemSettings()
  return (
    <Page>
      <PageHeader title={t('settings.title')} description={t('settings.description')} />
      {query.isPending ? (
        <PageLoader />
      ) : query.isError ? (
        <ErrorState onRetry={() => void query.refetch()} />
      ) : (
        <SettingsForm key={query.data.updated_at ?? 'default'} settings={query.data} />
      )}
    </Page>
  )
}

function SettingsForm({ settings }: { settings: LineItemSettings }) {
  const { t } = useTranslation('line_items')
  const { hasPermission } = useSession()
  const canEdit = hasPermission('settings.manage')
  const queryClient = useQueryClient()
  const [mode, setMode] = useState<AmountMode>(settings.amount_mode)
  const [saving, setSaving] = useState(false)
  const dirty = mode !== settings.amount_mode

  const save = async () => {
    setSaving(true)
    try {
      const saved = await settingsApi.updateLineItemSettings({ amount_mode: mode }, settings.updated_at)
      queryClient.setQueryData(settingsKeys.settings(), saved)
      await queryClient.invalidateQueries({ queryKey: ['line-items'] })
      notifySuccess(t('settings.saved'))
    } catch (error) {
      if (isApiError(error) && error.code === 'version_conflict') {
        notifyError(error, { title: t('settings.conflict') })
        await queryClient.invalidateQueries({ queryKey: settingsKeys.settings() })
      } else {
        notifyError(error)
      }
    } finally {
      setSaving(false)
    }
  }

  return (
    <form
      className="grid max-w-2xl gap-6"
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      <UnsavedChangesGuard when={canEdit && dirty} />
      <fieldset className="grid gap-3" disabled={!canEdit || saving}>
        <legend className="mb-2 text-base font-semibold">{t('settings.amountMode')}</legend>
        {DEAL_AMOUNT_MODES.map((value) => (
          <OptionCard
            key={value}
            type="radio"
            name="amount_mode"
            value={value}
            checked={mode === value}
            onChange={() => setMode(value)}
            title={t(`settings.modes.${value}`)}
            description={t(`settings.modeHints.${value}`)}
            disabled={!canEdit || saving}
          />
        ))}
      </fieldset>
      {mode === 'items' && dirty && <Notice>{t('settings.switchHint')}</Notice>}
      {canEdit && (
        <div className="flex justify-end">
          <Button type="submit" disabled={saving || !dirty}>
            {t('settings.save')}
          </Button>
        </div>
      )}
    </form>
  )
}
