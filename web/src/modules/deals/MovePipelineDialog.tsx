import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { activePipelines, usePipelines } from '@/modules/pipelines/api'
import type { Pipeline, StageKind } from '@/modules/pipelines/types'
import type { Deal } from './types'
import { NativeSelect } from '@/components/ui/native-select'

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
  const pipelineSelectId = useId()
  const stageSelectId = useId()

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        if (pipeline && stage) {
          onConfirm({ pipelineId: pipeline.id, stageId: stage.id, stageKind: stage.kind, stageName: stage.name })
        }
      }}
    >
      <DialogHeader>
        <DialogTitle>{t('move.title')}</DialogTitle>
        <DialogDescription>
          {deal ? t('move.descriptionFor', { title: deal.title }) : t('move.description')}
        </DialogDescription>
      </DialogHeader>
      {targets.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t('move.noTargets')}</p>
      ) : (
        <>
          <div className="grid gap-2">
            <Label htmlFor={pipelineSelectId}>{t('move.pipeline')}</Label>
            <NativeSelect
              id={pipelineSelectId}
              value={pipelineId}
              onChange={(event) => {
                const next = targets.find((item) => item.id === event.target.value)
                setPipelineId(event.target.value)
                setStageId(defaultStageId(next, stageKindOf(deal)))
              }}
              className="h-9 w-full rounded-md border bg-background px-3 text-sm"
            >
              {targets.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </NativeSelect>
          </div>
          <div className="grid gap-2">
            <Label htmlFor={stageSelectId}>{t('move.stage')}</Label>
            <NativeSelect
              id={stageSelectId}
              value={stageId}
              onChange={(event) => setStageId(event.target.value)}
              className="h-9 w-full rounded-md border bg-background px-3 text-sm"
            >
              {stages.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </NativeSelect>
          </div>
        </>
      )}
      <DialogFooter>
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={pending}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" disabled={pending || !pipeline || !stage}>
          {t('move.confirm')}
        </Button>
      </DialogFooter>
    </form>
  )
}

export function MovePipelineDialog({ open, onOpenChange, ...props }: MovePipelineDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>{open && <MovePipelineForm {...props} onOpenChange={onOpenChange} />}</DialogContent>
    </Dialog>
  )
}
