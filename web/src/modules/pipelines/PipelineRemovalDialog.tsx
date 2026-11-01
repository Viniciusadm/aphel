import { DialogForm } from '@/components/ui/dialog-form'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import type { PipelineRemovalBody } from './api'
import type { Pipeline } from './types'
import { compatibleStages, dealCount, defaultMapping } from './removal'
import { NativeSelect } from '@/components/ui/native-select'
import { Field } from '@/components/ui/field'

export interface PipelineRemovalDialogProps {
  pipeline: Pipeline | null
  pipelines: Pipeline[]
  pending?: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: (body: PipelineRemovalBody) => void
}

function RemovalForm({
  pipeline,
  pipelines,
  pending,
  onOpenChange,
  onConfirm,
}: Omit<PipelineRemovalDialogProps, 'pipeline'> & { pipeline: Pipeline }) {
  const { t } = useTranslation('pipelines')
  const deals = dealCount(pipeline)
  const targets = pipelines.filter((item) => item.id !== pipeline.id && item.is_active)
  const [targetId, setTargetId] = useState(targets[0]?.id ?? '')
  const target = targets.find((item) => item.id === targetId)
  const [mapping, setMapping] = useState<Record<string, string>>(() => defaultMapping(pipeline, target))
  const baseId = useId()
  const sources = pipeline.stages.filter((stage) => (stage.deal_count ?? 0) > 0)
  const unmapped = sources.filter((stage) => !mapping[stage.id])
  const blocked = deals > 0 && (!target || unmapped.length > 0)

  return (
    <DialogForm
      submitVariant="destructive"
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        if (blocked) return
        onConfirm(
          deals > 0
            ? { target_pipeline_id: targetId, stage_mapping: mapping }
            : { target_pipeline_id: targetId || undefined },
        )
      }}
      title={<>{t('remove.title', { name: pipeline.name })}</>}
      description={<>{deals > 0 ? t('remove.withDeals', { count: deals }) : t('remove.description')}</>}
      cancelLabel={t('actions.cancel')}
      submitLabel={t('remove.confirm')}
      onCancel={() => onOpenChange(false)}
      pending={pending}
      submitDisabled={pending || blocked}
    >
      {deals > 0 &&
        (targets.length === 0 ? (
          <SharedFieldError>
            {t('remove.noTarget')}
          </SharedFieldError>
        ) : (
          <>
            <Field label={t('remove.target')}>
              {(control) => <NativeSelect
                {...control}
                value={targetId}
                onChange={(event) => {
                  const next = targets.find((item) => item.id === event.target.value)
                  setTargetId(event.target.value)
                  setMapping(defaultMapping(pipeline, next))
                }}
                className="w-full"
              >
                {targets.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.name}
                  </option>
                ))}
              </NativeSelect>}
            </Field>
            <fieldset className="grid gap-2">
              <legend className="text-sm font-medium">{t('remove.mapping')}</legend>
              {sources.map((stage) => {
                const id = `${baseId}-${stage.id}`
                const options = compatibleStages(target, stage)
                return (
                  <Field
                    key={stage.id}
                    id={id}
                    label={t('remove.stageLabel', { name: stage.name, count: stage.deal_count ?? 0 })}
                    className="sm:grid-cols-2 sm:items-center sm:gap-3"
                  >
                    <NativeSelect
                      id={id}
                      value={mapping[stage.id] ?? ''}
                      onChange={(event) => setMapping((current) => ({ ...current, [stage.id]: event.target.value }))}
                      className="w-full"
                    >
                      {options.length === 0 && <option value="">{t('remove.noCompatible')}</option>}
                      {options.map((option) => (
                        <option key={option.id} value={option.id}>
                          {option.name}
                        </option>
                      ))}
                    </NativeSelect>
                  </Field>
                )
              })}
            </fieldset>
          </>
        ))}
    </DialogForm>
  )
}

export function PipelineRemovalDialog({ pipeline, onOpenChange, ...props }: PipelineRemovalDialogProps) {
  return (
    <Dialog open={pipeline !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        {pipeline && <RemovalForm {...props} pipeline={pipeline} onOpenChange={onOpenChange} />}
      </DialogContent>
    </Dialog>
  )
}
