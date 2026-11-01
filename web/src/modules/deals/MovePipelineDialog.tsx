import { DialogForm } from '@/components/ui/dialog-form'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import { activePipelines, usePipelines } from '@/modules/pipelines/api'
import type { Pipeline, StageKind } from '@/modules/pipelines/types'
import type { Deal } from './types'
import { NativeSelect } from '@/components/ui/native-select'
import { Field } from '@/components/ui/field'

export interface PipelineMoveChoice {
  pipelineId: string
  stageId: string
  stageKind: StageKind
  stageName: string
}

export interface MovePipelineDialogProps {
  open: boolean
  deal?: Deal
  pending?: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: (choice: PipelineMoveChoice) => void
}

function stageKindOf(deal: Deal | undefined): StageKind {
  return deal?.stage.kind ?? 'open'
}

function defaultStageId(pipeline: Pipeline | undefined, kind: StageKind): string {
  const stages = pipeline?.stages.filter((stage) => stage.is_active) ?? []
  return (stages.find((stage) => stage.kind === kind) ?? stages[0])?.id ?? ''
}

function MovePipelineForm({ deal, pending, onOpenChange, onConfirm }: Omit<MovePipelineDialogProps, 'open'>) {
  const { t } = useTranslation('deals', { keyPrefix: 'pipeline' })
  const pipelines = usePipelines()
  const targets = activePipelines(pipelines.data).filter((pipeline) => pipeline.id !== deal?.pipeline_id)
  const [pipelineId, setPipelineId] = useState(targets[0]?.id ?? '')
  const pipeline = targets.find((item) => item.id === pipelineId)
  const [stageId, setStageId] = useState(() => defaultStageId(pipeline, stageKindOf(deal)))
  const stages = pipeline?.stages.filter((stage) => stage.is_active) ?? []
  const stage = stages.find((item) => item.id === stageId)

  return (
    <DialogForm
      onSubmit={(event) => {
        event.preventDefault()
        if (pipeline && stage) {
          onConfirm({ pipelineId: pipeline.id, stageId: stage.id, stageKind: stage.kind, stageName: stage.name })
        }
      }}
      title={t('move.title')}
      description={<>{deal ? t('move.descriptionFor', { title: deal.title }) : t('move.description')}</>}
      cancelLabel={t('actions.cancel')}
      submitLabel={t('move.confirm')}
      onCancel={() => onOpenChange(false)}
      pending={pending}
      submitDisabled={pending || !pipeline || !stage}
    >
      {targets.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t('move.noTargets')}</p>
      ) : (
        <>
          <Field label={t('move.pipeline')}>
            {(control) => <NativeSelect
              {...control}
              value={pipelineId}
              onChange={(event) => {
                const next = targets.find((item) => item.id === event.target.value)
                setPipelineId(event.target.value)
                setStageId(defaultStageId(next, stageKindOf(deal)))
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
          <Field label={t('move.stage')}>
            {(control) => <NativeSelect
              {...control}
              value={stageId}
              onChange={(event) => setStageId(event.target.value)}
              className="w-full"
            >
              {stages.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </NativeSelect>}
          </Field>
        </>
      )}
    </DialogForm>
  )
}

export function MovePipelineDialog({ open, onOpenChange, ...props }: MovePipelineDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>{open && <MovePipelineForm {...props} onOpenChange={onOpenChange} />}</DialogContent>
    </Dialog>
  )
}
