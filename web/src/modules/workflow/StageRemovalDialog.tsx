import { DialogForm } from '@/components/ui/dialog-form'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import type { PipelineStage } from './types'
import { NativeSelect } from '@/components/ui/native-select'
import { Field } from '@/components/ui/field'

export type StageRemovalMode = 'deactivate' | 'delete'

export interface StageRemovalDialogProps {
  mode: StageRemovalMode
  stage: PipelineStage | null
  stages: PipelineStage[]
  pending?: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: (targetStageId?: string) => void
}

function removalTargets(stage: PipelineStage, stages: PipelineStage[]): PipelineStage[] {
  return stages.filter((candidate) => candidate.id !== stage.id && candidate.is_active && candidate.kind === stage.kind)
}

function RemovalForm({
  mode,
  stage,
  stages,
  pending,
  onOpenChange,
  onConfirm,
}: Omit<StageRemovalDialogProps, 'stage'> & { stage: PipelineStage }) {
  const { t } = useTranslation('workflow', { keyPrefix: 'editor' })
  const records = stage.record_count ?? 0
  const targets = removalTargets(stage, stages)
  const [target, setTarget] = useState(targets[0]?.id ?? '')
  const needsTarget = records > 0
  const blocked = needsTarget && targets.length === 0

  return (
    <DialogForm
      submitVariant="destructive"
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        if (blocked) return
        onConfirm(needsTarget ? target : undefined)
      }}
      title={<>{t(`settings.removal.${mode}Title`, { name: stage.name })}</>}
      description={<>{needsTarget ? t('settings.removal.withRecords', { count: records }) : t(`settings.removal.${mode}Description`)}</>}
      cancelLabel={t('actions.cancel')}
      submitLabel={<>{t(`settings.removal.${mode}Confirm`)}</>}
      onCancel={() => onOpenChange(false)}
      pending={pending}
      submitDisabled={pending || blocked}
    >
      {needsTarget &&
        (blocked ? (
          <SharedFieldError>
            {t('settings.removal.noTarget')}
          </SharedFieldError>
        ) : (
          <Field label={t('settings.removal.target')}>
            {(control) => <NativeSelect
              {...control}
              value={target}
              onChange={(event) => setTarget(event.target.value)}
              className="w-full"
            >
              {targets.map((candidate) => (
                <option key={candidate.id} value={candidate.id}>
                  {candidate.name}
                </option>
              ))}
            </NativeSelect>}
          </Field>
        ))}
    </DialogForm>
  )
}

export function StageRemovalDialog({ stage, onOpenChange, ...props }: StageRemovalDialogProps) {
  return (
    <Dialog open={stage !== null} onOpenChange={onOpenChange}>
      <DialogContent>{stage && <RemovalForm {...props} stage={stage} onOpenChange={onOpenChange} />}</DialogContent>
    </Dialog>
  )
}
