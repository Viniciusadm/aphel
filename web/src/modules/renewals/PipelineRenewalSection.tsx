import { useEffect, useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback/notify'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { NumberMaskInput } from '@/components/ui/number-mask-input'
import { ToggleField } from '@/components/ui/toggle-field'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect, WIDE_FIELD, type Pipeline } from '@/sdk'
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
  const formId = useId()
  const [state, setState] = useState({ dirty: false, saving: false, valid: true })

  return (
    <Card>
      <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="grid gap-1.5">
          <CardTitle>{t('renewal.title')}</CardTitle>
          <CardDescription>{t('renewal.description')}</CardDescription>
        </div>
        {settings.isSuccess && (
          <Button type="submit" form={formId} size="sm" disabled={!state.dirty || state.saving || !state.valid}>
            {t('renewal.save')}
          </Button>
        )}
      </CardHeader>
      <CardContent>
        {settings.isPending ? (
          <div className="grid gap-4 sm:grid-cols-2" aria-busy="true">
            <Skeleton className="h-16 w-full sm:col-span-2" />
            <Skeleton className="h-16 w-full" />
            <Skeleton className="h-16 w-full" />
            <Skeleton className="h-16 w-full" />
            <Skeleton className="h-16 w-full" />
          </div>
        ) : settings.isError ? (
          <ErrorState onRetry={() => void settings.refetch()} />
        ) : (
          <RenewalForm
            key={settings.data.updated_at ?? 'new'}
            formId={formId}
            pipelineId={pipeline.id}
            settings={settings.data}
            workflow={workflow}
            onState={setState}
          />
        )}
      </CardContent>
    </Card>
  )
}

interface RenewalFormProps {
  formId: string
  pipelineId: string
  settings: RenewalSettings
  workflow: Pipeline[]
  onState: (state: { dirty: boolean; saving: boolean; valid: boolean }) => void
}

function RenewalForm({ formId, pipelineId, settings, workflow, onState }: RenewalFormProps) {
  const { t } = useTranslation('renewals')
  const save = useSaveRenewalSettings(pipelineId)
  const [initial] = useState(() => JSON.stringify(draftOf(settings)))
  const [draft, setDraft] = useState<Draft>(() => draftOf(settings))
  const [submitted, setSubmitted] = useState(false)
  const stages = openStages(workflow, draft.target_pipeline_id)
  const days = Number(draft.days_before)
  const daysValid = draft.days_before !== '' && Number.isInteger(days) && days >= 0 && days <= MAX_DAYS_BEFORE
  const stageValid = stages.some((stage) => stage.id === draft.target_stage_id)
  const dirty = JSON.stringify(draft) !== initial
  const update = (patch: Partial<Draft>) => setDraft((current) => ({ ...current, ...patch }))

  useEffect(() => onState({ dirty, saving: save.isPending, valid: true }), [dirty, save.isPending, onState])
  useEffect(() => () => onState({ dirty: false, saving: false, valid: true }), [onState])

  const submit = (event: FormEvent) => {
    event.preventDefault()
    setSubmitted(true)
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
    <form id={formId} className="grid gap-4 sm:grid-cols-2" noValidate onSubmit={submit}>
      <ToggleField
        className={WIDE_FIELD}
        type="switch"
        label={t('renewal.enabled')}
        description={t('renewal.enabledHint')}
        checked={draft.is_enabled}
        onCheckedChange={(is_enabled) => update({ is_enabled })}
      />
      <Field
        label={t('renewal.daysBefore')}
        description={t('renewal.daysBeforeHint', { max: MAX_DAYS_BEFORE })}
        error={submitted && !daysValid ? t('renewal.daysInvalid', { max: MAX_DAYS_BEFORE }) : undefined}
      >
        {(control) => (
          <NumberMaskInput
            {...control}
            kind="integer"
            maxLength={3}
            className="w-32"
            placeholder={t('placeholders.days')}
            value={draft.days_before}
            onValueChange={(days_before) => update({ days_before })}
          />
        )}
      </Field>
      <Field label={t('renewal.owner')}>
        {(control) => (
          <NativeSelect
            {...control}
            value={draft.owner_mode}
            onChange={(event) => update({ owner_mode: event.target.value as RenewalOwnerMode })}
          >
            {OWNER_MODES.map((mode) => (
              <option key={mode} value={mode}>
                {t(`renewal.ownerModes.${mode}`)}
              </option>
            ))}
          </NativeSelect>
        )}
      </Field>
      <Field label={t('renewal.targetPipeline')}>
        {(control) => (
          <NativeSelect
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
          </NativeSelect>
        )}
      </Field>
      <Field
        label={t('renewal.targetStage')}
        error={submitted && !stageValid ? t('renewal.noStage') : undefined}
      >
        {(control) => (
          <NativeSelect
            {...control}
            value={draft.target_stage_id}
            onChange={(event) => update({ target_stage_id: event.target.value })}
          >
            {!stageValid && <option value="">{t('renewal.noStage')}</option>}
            {stages.map((stage) => (
              <option key={stage.id} value={stage.id}>
                {stage.name}
              </option>
            ))}
          </NativeSelect>
        )}
      </Field>
      <Field label={t('renewal.titlePrefix')} description={t('renewal.titlePrefixHint')} className={WIDE_FIELD}>
        {(control) => (
          <Input
            {...control}
            value={draft.title_prefix}
            maxLength={60}
            placeholder={t('placeholders.titlePrefix')}
            onChange={(event) => update({ title_prefix: event.target.value })}
          />
        )}
      </Field>
    </form>
  )
}
