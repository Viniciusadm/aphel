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
import type { PipelineRemovalBody } from './api'
import type { Pipeline } from './types'
import { compatibleStages, dealCount, defaultMapping } from './removal'

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
  const targetSelectId = useId()
  const baseId = useId()
  const sources = pipeline.stages.filter((stage) => (stage.deal_count ?? 0) > 0)
  const unmapped = sources.filter((stage) => !mapping[stage.id])
  const blocked = deals > 0 && (!target || unmapped.length > 0)

  return (
    <form
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
    >
      <DialogHeader>
        <DialogTitle>{t('remove.title', { name: pipeline.name })}</DialogTitle>
        <DialogDescription>
          {deals > 0 ? t('remove.withDeals', { count: deals }) : t('remove.description')}
        </DialogDescription>
      </DialogHeader>
      {deals > 0 &&
        (targets.length === 0 ? (
          <p role="alert" className="text-sm text-destructive">
            {t('remove.noTarget')}
          </p>
        ) : (
          <>
            <div className="grid gap-2">
              <Label htmlFor={targetSelectId}>{t('remove.target')}</Label>
              <select
                id={targetSelectId}
                value={targetId}
                onChange={(event) => {
                  const next = targets.find((item) => item.id === event.target.value)
                  setTargetId(event.target.value)
                  setMapping(defaultMapping(pipeline, next))
                }}
                className="h-9 w-full rounded-md border bg-background px-3 text-sm"
              >
                {targets.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.name}
                  </option>
                ))}
              </select>
            </div>
            <fieldset className="grid gap-2">
              <legend className="text-sm font-medium">{t('remove.mapping')}</legend>
              {sources.map((stage) => {
                const id = `${baseId}-${stage.id}`
                const options = compatibleStages(target, stage)
                return (
                  <div key={stage.id} className="grid gap-1 sm:grid-cols-2 sm:items-center sm:gap-3">
                    <Label htmlFor={id} className="font-normal">
                      {t('remove.stageLabel', { name: stage.name, count: stage.deal_count ?? 0 })}
                    </Label>
                    <select
                      id={id}
                      value={mapping[stage.id] ?? ''}
                      onChange={(event) => setMapping((current) => ({ ...current, [stage.id]: event.target.value }))}
                      className="h-9 w-full rounded-md border bg-background px-3 text-sm"
                    >
                      {options.length === 0 && <option value="">{t('remove.noCompatible')}</option>}
                      {options.map((option) => (
                        <option key={option.id} value={option.id}>
                          {option.name}
                        </option>
                      ))}
                    </select>
                  </div>
                )
              })}
            </fieldset>
          </>
        ))}
      <DialogFooter>
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={pending}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" variant="destructive" disabled={pending || blocked}>
          {t('remove.confirm')}
        </Button>
      </DialogFooter>
    </form>
  )
}

export function PipelineRemovalDialog({ pipeline, onOpenChange, ...props }: PipelineRemovalDialogProps) {
  return (
    <Dialog open={pipeline !== null} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto">
        {pipeline && <RemovalForm {...props} pipeline={pipeline} onOpenChange={onOpenChange} />}
      </DialogContent>
    </Dialog>
  )
}
