import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess, PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { usePipelines } from '@/modules/pipelines/api'
import { useQuoteMutations, useQuoteSettings } from './api'
import type { QuoteSettings } from './types'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'
import { Field } from '@/components/ui/field'

const NO_MOVE = '__none__'

export function QuoteSettingsPage() {
  const { t } = useTranslation('quotes')
  const query = useQuoteSettings()
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

function SettingsForm({ settings }: { settings: QuoteSettings }) {
  const { t } = useTranslation('quotes')
  const ids = { days: useId(), terms: useId() }
  const { hasPermission } = useSession()
  const canEdit = hasPermission('settings.manage')
  const pipelines = usePipelines()
  const mutations = useQuoteMutations()
  const [days, setDays] = useState(settings.default_valid_days?.toString() ?? '')
  const [terms, setTerms] = useState(settings.default_terms ?? '')
  const [stages, setStages] = useState<Record<string, string>>(settings.accept_stages)
  const [dirty, setDirty] = useState(false)

  const save = async () => {
    const parsed = days.trim() === '' ? null : Number.parseInt(days, 10)
    try {
      await mutations.updateSettings.mutateAsync({
        request: {
          default_valid_days: Number.isNaN(parsed) ? null : parsed,
          default_terms: terms.trim() || null,
          accept_stages: stages,
        },
        version: settings.updated_at,
      })
      setDirty(false)
      notifySuccess(t('settings.saved'))
    } catch (error) {
      notifyError(error)
    }
  }

  return (
    <div className="grid max-w-2xl gap-6">
      <UnsavedChangesGuard when={dirty} />
      <div className="grid gap-2.5">
        <Label htmlFor={ids.days}>{t('settings.validDays')}</Label>
        <Input placeholder={t('placeholders.days')}
          id={ids.days}
          className="w-32"
          inputMode="numeric"
          value={days}
          disabled={!canEdit}
          onChange={(event) => {
            setDays(event.target.value)
            setDirty(true)
          }}
        />
        <p className="text-xs text-muted-foreground">{t('settings.validDaysHint')}</p>
      </div>
      <Field label={t('settings.terms')}>
        {(control) => <Textarea placeholder={t('placeholders.terms')}
          {...control}
          rows={4}
          value={terms}
          disabled={!canEdit}
          onChange={(event) => {
            setTerms(event.target.value)
            setDirty(true)
          }}
        />}
      </Field>
      <div className="grid gap-3">
        <div className="grid gap-1">
          <h3 className="text-sm font-medium">{t('settings.acceptStages')}</h3>
          <p className="text-xs text-muted-foreground">{t('settings.acceptStagesHint')}</p>
        </div>
        {(pipelines.data ?? []).map((pipeline) => (
          <div key={pipeline.id} className="grid gap-1.5 sm:grid-cols-[12rem_minmax(0,1fr)] sm:items-center">
            <Label>{pipeline.name}</Label>
            <Select
              value={stages[pipeline.id] ?? NO_MOVE}
              disabled={!canEdit}
              onValueChange={(value) => {
                setStages((current) => {
                  const next = { ...current }
                  if (value === NO_MOVE) delete next[pipeline.id]
                  else next[pipeline.id] = value
                  return next
                })
                setDirty(true)
              }}
            >
              <SelectTrigger aria-label={pipeline.name}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={NO_MOVE}>{t('settings.noMove')}</SelectItem>
                {pipeline.stages.map((stage) => (
                  <SelectItem key={stage.id} value={stage.id}>
                    {stage.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        ))}
      </div>
      {canEdit && (
        <Button className="w-fit" onClick={() => void save()} disabled={!dirty || mutations.updateSettings.isPending}>
          {t('settings.save')}
        </Button>
      )}
    </div>
  )
}
