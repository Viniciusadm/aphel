import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess, PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { SectionHeader } from '@/features/settings/components/SectionHeader'
import { DEAL_AMOUNT_MODES, productsApi, productsKeys, useDealItemSettings, type DealAmountMode, type DealItemSettings } from '../api'

export function DealItemSettingsPage() {
  const { t } = useTranslation('products')
  const query = useDealItemSettings()
  return (
    <section className="flex flex-col gap-6">
      <SectionHeader title={t('settings.title')} description={t('settings.description')} />
      {query.isPending ? (
        <PageLoader />
      ) : query.isError ? (
        <ErrorState onRetry={() => void query.refetch()} />
      ) : (
        <SettingsForm key={query.data.updated_at ?? 'default'} settings={query.data} />
      )}
    </section>
  )
}

function SettingsForm({ settings }: { settings: DealItemSettings }) {
  const { t } = useTranslation('products')
  const { hasPermission } = useSession()
  const canEdit = hasPermission('settings.manage')
  const queryClient = useQueryClient()
  const [mode, setMode] = useState<DealAmountMode>(settings.amount_mode)
  const [saving, setSaving] = useState(false)
  const dirty = mode !== settings.amount_mode

  const save = async () => {
    setSaving(true)
    try {
      const saved = await productsApi.updateDealItemSettings({ amount_mode: mode }, settings.updated_at)
      queryClient.setQueryData(productsKeys.settings(), saved)
      await queryClient.invalidateQueries({ queryKey: ['deal-items'] })
      notifySuccess(t('settings.saved'))
    } catch (error) {
      if (isApiError(error) && error.code === 'version_conflict') {
        notifyError(error, { title: t('settings.conflict') })
        await queryClient.invalidateQueries({ queryKey: productsKeys.settings() })
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
          <label
            key={value}
            className="flex cursor-pointer items-start gap-3 rounded-md border p-3 has-[:checked]:border-primary"
          >
            <input
              type="radio"
              name="amount_mode"
              value={value}
              checked={mode === value}
              onChange={() => setMode(value)}
              className="mt-1 accent-[var(--primary)]"
            />
            <span className="grid gap-0.5">
              <span className="text-sm font-medium">{t(`settings.modes.${value}`)}</span>
              <span className="text-xs text-muted-foreground">{t(`settings.modeHints.${value}`)}</span>
            </span>
          </label>
        ))}
      </fieldset>
      {mode === 'items' && <p className="text-sm text-muted-foreground">{t('settings.switchHint')}</p>}
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
