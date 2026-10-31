import { useMutation } from '@tanstack/react-query'
import { FlaskConical, Play } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { RecordFieldInput } from '@/components/fields'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import type { FieldSchema } from '@/features/records/types'
import { automationsApi, useAutomationCache, useAutomationVersions } from '../api'
import { StepTimeline } from '../components/StepTimeline'
import { useAutomationLabels } from '../lib/labels'
import type { Automation, AutomationCatalog, DryRunResult, StepSpec } from '../types'
import { EmptyState } from '@/components/ui/empty-state'
import { Field } from '@/components/ui/field'

export interface DryRunPanelProps {
  automation: Automation
  catalog: AutomationCatalog
  schema: FieldSchema | undefined
  draftSteps: StepSpec[]
  dirty: boolean
  onSaveDraft: () => Promise<Automation | null>
}

export function DryRunPanel({ automation, catalog, schema, draftSteps, dirty, onSaveDraft }: DryRunPanelProps) {
  const { t } = useTranslation('automations')
  const draftId = useId()
  const labels = useAutomationLabels(catalog)
  const cache = useAutomationCache()
  const versions = useAutomationVersions(automation.id)
  const neverPublished = automation.version === 0
  const [record, setRecord] = useState<string | null>(null)
  const [useDraft, setUseDraft] = useState(true)
  const [result, setResult] = useState<{ data: DryRunResult; usedDraft: boolean } | null>(null)
  const draftMode = neverPublished || useDraft
  const triggerEntry = catalog.triggers.find((entry) => entry.key === automation.trigger.type)

  const dryRun = useMutation({
    mutationFn: async () => {
      if (!record) return null
      if (draftMode && dirty) {
        const saved = await onSaveDraft()
        if (!saved) return null
      }
      const data = await automationsApi.dryRun(automation.id, { record_id: record, use_draft: draftMode })
      return { data, usedDraft: draftMode }
    },
    onSuccess: (value) => {
      if (value) setResult(value)
    },
    onError: (error) => notifyError(error),
  })

  const manualRun = useMutation({
    mutationFn: () => automationsApi.run(automation.id, record!),
    onSuccess: () => {
      notifySuccess(t('test.queued'))
      void cache.invalidateRuns(automation.id)
    },
    onError: (error) => notifyError(error),
  })

  const published = versions.data?.find((version) => version.version === automation.version)
  const definitionSteps = result?.usedDraft ? draftSteps : (published?.definition.steps ?? draftSteps)

  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,22rem)_minmax(0,1fr)]">
      <section className="grid content-start gap-4 rounded-lg border bg-card p-4">
        <div className="grid gap-1">
          <h2 className="font-medium">{t('test.title')}</h2>
          <p className="text-sm text-muted-foreground">{t('test.description')}</p>
        </div>
        <Field label={t('test.record', { type: labels.recordTypeLabel(automation.record_type) })}>
          {(fieldControl) => <RecordFieldInput id={fieldControl.id} targetRecordType={automation.record_type} value={record} onChange={setRecord} />}
        </Field>
        <div className="flex items-start gap-2">
          <Checkbox
            id={draftId}
            checked={draftMode}
            disabled={neverPublished}
            onCheckedChange={(checked) => setUseDraft(checked === true)}
          />
          <div className="grid gap-0.5">
            <Label htmlFor={draftId}>{t('test.useDraft')}</Label>
            <p className="text-xs text-muted-foreground">
              {neverPublished ? t('test.neverPublished') : dirty && draftMode ? t('test.willSave') : t('test.useDraftHint')}
            </p>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button type="button" disabled={!record || dryRun.isPending} onClick={() => dryRun.mutate()}>
            <FlaskConical aria-hidden="true" />
            {dryRun.isPending ? t('test.running') : t('test.run')}
          </Button>
          {triggerEntry?.manual && automation.status === 'active' && (
            <Button type="button" variant="outline" disabled={!record || manualRun.isPending} onClick={() => manualRun.mutate()}>
              <Play aria-hidden="true" />
              {t('test.runNow')}
            </Button>
          )}
        </div>
        <p className="text-xs text-muted-foreground">{t('test.noEffects')}</p>
      </section>
      <section className="grid content-start gap-3" aria-live="polite">
        {result ? (
          <>
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="font-medium">{t('test.result')}</h2>
              <Badge variant={result.data.outcome === 'failed' ? 'destructive' : result.data.outcome === 'completed' ? 'success' : 'warning'}>
                {t(`test.outcomes.${result.data.outcome}` as 'test.outcomes.completed', { defaultValue: result.data.outcome })}
              </Badge>
              <span className="text-xs text-muted-foreground">
                {result.usedDraft ? t('test.ranDraft') : t('test.ranVersion', { version: result.data.version ?? automation.version })}
              </span>
            </div>
            {!result.data.record_available && <p className="text-sm text-muted-foreground">{t('test.recordUnavailable')}</p>}
            {result.data.error && (
              <p role="alert" className="rounded-md bg-destructive/10 px-3 py-2 text-sm text-destructive">
                {result.data.error}
              </p>
            )}
            <StepTimeline steps={result.data.steps} definitionSteps={definitionSteps} catalog={catalog} schema={schema} />
          </>
        ) : (
          <EmptyState title={t('test.empty')} />
        )}
      </section>
    </div>
  )
}
