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
import type { PipelineStage } from './types'

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
  const { t } = useTranslation('pipelines', { keyPrefix: 'editor' })
  const deals = stage.deal_count ?? 0
  const targets = removalTargets(stage, stages)
  const [target, setTarget] = useState(targets[0]?.id ?? '')
  const selectId = useId()
  const needsTarget = deals > 0
  const blocked = needsTarget && targets.length === 0

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        if (blocked) return
        onConfirm(needsTarget ? target : undefined)
      }}
    >
      <DialogHeader>
        <DialogTitle>{t(`settings.removal.${mode}Title`, { name: stage.name })}</DialogTitle>
        <DialogDescription>
          {needsTarget ? t('settings.removal.withDeals', { count: deals }) : t(`settings.removal.${mode}Description`)}
        </DialogDescription>
      </DialogHeader>
      {needsTarget &&
        (blocked ? (
          <p role="alert" className="text-sm text-destructive">
            {t('settings.removal.noTarget')}
          </p>
        ) : (
          <div className="grid gap-2">
            <Label htmlFor={selectId}>{t('settings.removal.target')}</Label>
            <select
              id={selectId}
              value={target}
              onChange={(event) => setTarget(event.target.value)}
              className="h-9 w-full rounded-md border bg-background px-3 text-sm"
            >
              {targets.map((candidate) => (
                <option key={candidate.id} value={candidate.id}>
                  {candidate.name}
                </option>
              ))}
            </select>
          </div>
        ))}
      <DialogFooter>
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={pending}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" variant="destructive" disabled={pending || blocked}>
          {t(`settings.removal.${mode}Confirm`)}
        </Button>
      </DialogFooter>
    </form>
  )
}

export function StageRemovalDialog({ stage, onOpenChange, ...props }: StageRemovalDialogProps) {
  return (
    <Dialog open={stage !== null} onOpenChange={onOpenChange}>
      <DialogContent>{stage && <RemovalForm {...props} stage={stage} onOpenChange={onOpenChange} />}</DialogContent>
    </Dialog>
  )
}
