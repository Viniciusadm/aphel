import { useId, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { toast } from '@/components/ui/toast-store'
import { dealPath } from '@/modules/deals/resource'
import { defaultPipeline, usePipelines } from '@/modules/pipelines/api'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useLeadActions } from './api'
import type { LeadTarget } from './DisqualifyLeadDialog'
import { lifecycleErrorMessage } from './errors'

export interface ConvertLeadDialogProps {
  lead: LeadTarget | null
  onOpenChange: (open: boolean) => void
}

export function ConvertLeadDialog({ lead, onOpenChange }: ConvertLeadDialogProps) {
  const { t } = useTranslation('leads')
  return (
    <Dialog open={lead !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('convert.title')}</DialogTitle>
          <DialogDescription>{t('convert.description', { name: lead?.name ?? '' })}</DialogDescription>
        </DialogHeader>
        {lead && <ConvertForm key={lead.id} lead={lead} onClose={() => onOpenChange(false)} />}
      </DialogContent>
    </Dialog>
  )
}

function ConvertForm({ lead, onClose }: { lead: LeadTarget; onClose: () => void }) {
  const { t } = useTranslation('leads')
  const formId = useId()
  const navigate = useNavigate()
  const pipelines = usePipelines()
  const { convert } = useLeadActions()
  const activePipelines = useMemo(
    () => (pipelines.data ?? []).filter((pipeline) => pipeline.is_active),
    [pipelines.data],
  )
  const [pipelineChoice, setPipelineChoice] = useState<string | null>(null)
  const pipelineId = pipelineChoice ?? defaultPipeline(activePipelines)?.id ?? ''
  const pipeline = activePipelines.find((item) => item.id === pipelineId)
  const openStages = useMemo(
    () =>
      (pipeline?.stages ?? [])
        .filter((stage) => stage.is_active && stage.kind === 'open')
        .sort((left, right) => left.position - right.position),
    [pipeline],
  )
  const [stageId, setStageId] = useState('')
  const [title, setTitle] = useState(lead.name)
  const [amount, setAmount] = useState('')
  const [pipelineError, setPipelineError] = useState(false)

  const submit = async () => {
    if (!pipelineId) {
      setPipelineError(true)
      return
    }
    try {
      const result = await convert.mutateAsync({
        contactId: lead.id,
        body: {
          pipeline_id: pipelineId,
          stage_id: stageId || undefined,
          title: title.trim() || undefined,
          amount: amount.trim().replace(',', '.') || undefined,
        },
      })
      const dealId = result.deal_id
      notifySuccess(t('convert.done', { name: lead.name }))
      onClose()
      if (dealId) void navigate(dealPath(dealId))
    } catch (error) {
      toast({ title: lifecycleErrorMessage(error, t), variant: 'destructive' })
    }
  }

  return (
    <>
      <form
        id={formId}
        className="grid gap-4"
        noValidate
        onSubmit={(event) => {
          event.preventDefault()
          void submit()
        }}
      >
        <div className="grid gap-1.5">
          <Label htmlFor={`${formId}-pipeline`}>{t('convert.pipeline')}</Label>
          <NativeSelect
            id={`${formId}-pipeline`}
            value={pipelineId}
            disabled={pipelines.isPending}
            aria-invalid={pipelineError ? true : undefined}
            onChange={(event) => {
              setPipelineChoice(event.target.value)
              setStageId('')
              setPipelineError(false)
            }}
          >
            <option value="">{t('convert.pipelinePlaceholder')}</option>
            {activePipelines.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name}
              </option>
            ))}
          </NativeSelect>
          {pipelineError && <p className="text-xs text-destructive">{t('convert.pipelineRequired')}</p>}
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${formId}-stage`}>{t('convert.stage')}</Label>
          <NativeSelect
            id={`${formId}-stage`}
            value={stageId}
            disabled={!pipeline}
            onChange={(event) => setStageId(event.target.value)}
          >
            <option value="">{t('convert.firstStage')}</option>
            {openStages.map((stage) => (
              <option key={stage.id} value={stage.id}>
                {stage.name}
              </option>
            ))}
          </NativeSelect>
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${formId}-title`}>{t('convert.dealTitle')}</Label>
          <Input
            id={`${formId}-title`}
            value={title}
            maxLength={255}
            onChange={(event) => setTitle(event.target.value)}
          />
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${formId}-amount`}>{t('convert.amount')}</Label>
          <Input
            id={`${formId}-amount`}
            value={amount}
            inputMode="decimal"
            placeholder={t('convert.amountPlaceholder')}
            onChange={(event) => setAmount(event.target.value)}
          />
        </div>
      </form>
      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={convert.isPending}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={convert.isPending}>
          {t('convert.submit')}
        </Button>
      </DialogFooter>
    </>
  )
}
