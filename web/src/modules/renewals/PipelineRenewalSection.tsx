import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback/notify'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import type { Pipeline } from '@/modules/workflow/types'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useRenewalSettings, useSaveRenewalSettings } from './api'
import type { RenewalOwnerMode, RenewalSettings } from './types'
import { Field } from '@/components/ui/field'

const OWNER_MODES: RenewalOwnerMode[] = ['original', 'none']
const MAX_DAYS_BEFORE = 365

interface Draft {
  is_enabled: boolean
  days_before: string
  target_pipeline_id: string
  target_stage_id: string
  title_prefix: string
  owner_mode: RenewalOwnerMode
}

function draftOf(settings: RenewalSettings): Draft {
  return {
    is_enabled: settings.is_enabled,
    days_before: String(settings.days_before),
    target_pipeline_id: settings.target_pipeline_id,
    target_stage_id: settings.target_stage_id,
    title_prefix: settings.title_prefix,
    owner_mode: settings.owner_mode,
  }
}

function openStages(workflow: Pipeline[], pipelineId: string) {
  return (workflow.find((pipeline) => pipeline.id === pipelineId)?.stages ?? []).filter(
    (stage) => stage.kind === 'open' && stage.is_active,
  )
}

export interface PipelineRenewalSectionProps {
  pipeline: Pipeline
  workflow: Pipeline[]
}

export function PipelineRenewalSection({ pipeline, workflow }: PipelineRenewalSectionProps) {
  const { t } = useTranslation('renewals')
  const settings = useRenewalSettings(pipeline.id)

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('renewal.title')}</CardTitle>
        <CardDescription>{t('renewal.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        {settings.isPending ? (
          <Skeleton className="h-40 w-full" />
        ) : settings.isError ? (
          <ErrorState onRetry={() => void settings.refetch()} />
        ) : (
          <RenewalForm
            key={settings.data.updated_at ?? 'new'}
            pipelineId={pipeline.id}
            settings={settings.data}
            workflow={workflow}
          />
        )}
      </CardContent>
    </Card>
  )
}

interface RenewalFormProps {
  pipelineId: string
  settings: RenewalSettings
  workflow: Pipeline[]
}

function RenewalForm({ pipelineId, settings, workflow }: RenewalFormProps) {
  const { t } = useTranslation('renewals')
  const save = useSaveRenewalSettings(pipelineId)
  const [draft, setDraft] = useState<Draft>(() => draftOf(settings))
  const ids = {
    enabled: useId(),
    days: useId(),
    pipeline: useId(),
    stage: useId(),
    prefix: useId(),
    owner: useId(),
  }
  const stages = openStages(workflow, draft.target_pipeline_id)
  const days = Number(draft.days_before)
  const daysValid = Number.isInteger(days) && days >= 0 && days <= MAX_DAYS_BEFORE
  const stageValid = stages.some((stage) => stage.id === draft.target_stage_id)
  const update = (patch: Partial<Draft>) => setDraft((current) => ({ ...current, ...patch }))

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (!daysValid || !stageValid) return
    save.mutate(
      {
        body: {
          is_enabled: draft.is_enabled,
          days_before: days,
          target_pipeline_id: draft.target_pipeline_id,
          target_stage_id: draft.target_stage_id,
          title_prefix: draft.title_prefix,
          owner_mode: draft.owner_mode,
        },
        version: settings.updated_at,
      },
      {
        onSuccess: () => notifySuccess(t('renewal.saved')),
        onError: (error) => notifyError(error),
      },
    )
  }

  return (
    <form className="grid gap-4" onSubmit={submit}>
      <div className="flex items-start gap-2">
        <Checkbox
          id={ids.enabled}
          checked={draft.is_enabled}
          onCheckedChange={(checked) => update({ is_enabled: checked === true })}
        />
        <Field id={ids.enabled} label={t('renewal.enabled')}>
          <p className="text-xs text-muted-foreground">{t('renewal.enabledHint')}</p>
        </Field>
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        <Field label={t('renewal.daysBefore')}>
          {(fieldControl) => <>
            <Input placeholder={t('placeholders.days')}
            {...fieldControl}
            inputMode="numeric"
            value={draft.days_before}
            aria-invalid={!daysValid}
            onChange={(event) => update({ days_before: event.target.value })}
          />
          <p className="text-xs text-muted-foreground">{t('renewal.daysBeforeHint', { max: MAX_DAYS_BEFORE })}</p>
          </>}
        </Field>
        <Field label={t('renewal.owner')}>
          {(control) => <NativeSelect
            {...control}
            value={draft.owner_mode}
            onChange={(event) => update({ owner_mode: event.target.value as RenewalOwnerMode })}
          >
            {OWNER_MODES.map((mode) => (
              <option key={mode} value={mode}>
                {t(`renewal.ownerModes.${mode}`)}
              </option>
            ))}
          </NativeSelect>}
        </Field>
        <Field label={t('renewal.targetPipeline')}>
          {(control) => <NativeSelect
            {...control}
            value={draft.target_pipeline_id}
            onChange={(event) => {
              const targetPipelineId = event.target.value
              update({
                target_pipeline_id: targetPipelineId,
                target_stage_id: openStages(workflow, targetPipelineId)[0]?.id ?? '',
              })
            }}
          >
            {workflow.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name}
              </option>
            ))}
          </NativeSelect>}
        </Field>
        <Field label={t('renewal.targetStage')}>
          {(control) => <NativeSelect
            {...control}
            value={draft.target_stage_id}
            aria-invalid={!stageValid}
            onChange={(event) => update({ target_stage_id: event.target.value })}
          >
            {!stageValid && <option value="">{t('renewal.noStage')}</option>}
            {stages.map((stage) => (
              <option key={stage.id} value={stage.id}>
                {stage.name}
              </option>
            ))}
          </NativeSelect>}
        </Field>
        <Field label={t('renewal.titlePrefix')} className="sm:col-span-2">
          {(control) => <>
            <Input placeholder={t('placeholders.titlePrefix')}
            {...control}
            value={draft.title_prefix}
            maxLength={60}
            onChange={(event) => update({ title_prefix: event.target.value })}
          />
          <p className="text-xs text-muted-foreground">{t('renewal.titlePrefixHint')}</p>
          </>}
        </Field>
      </div>
      <div className="flex justify-end">
        <Button type="submit" disabled={save.isPending || !daysValid || !stageValid}>
          {t('renewal.save')}
        </Button>
      </div>
    </form>
  )
}
