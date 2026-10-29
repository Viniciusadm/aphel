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
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import type { PipelineStage, StageKind } from './types'
import { useFieldSchema } from '@/features/records/api'
import { activeFields } from '@/features/records/fields'
import { CheckboxList, type CheckboxListItem } from './CheckboxList'
import { StageGuidanceEditor } from './guidance/StageGuidanceEditor'
import { REQUIRABLE_SYSTEM_FIELDS } from './requirements'
import { DEAL_RECORD_TYPE } from './resource'
import { parseRottingDays, stageRottingDays } from './rotting/lib'
import { RottingDaysField } from './rotting/RottingDaysField'

export interface StageFormValues {
  name: string
  probability: number
  kind: StageKind
  rotting_days: number | null
  entry_required_fields: string[]
  exit_required_fields: string[]
}

const SYSTEM_REQUIRABLE = new Set<string>(REQUIRABLE_SYSTEM_FIELDS)

function useRequirableFields(): CheckboxListItem[] {
  const schema = useFieldSchema(DEAL_RECORD_TYPE)
  if (!schema.data) return []
  return activeFields(schema.data)
    .filter((field) => (field.is_system ? SYSTEM_REQUIRABLE.has(field.key) : true))
    .map((field) => ({ value: field.key, label: field.label }))
}

export interface StageDialogProps {
  open: boolean
  stage?: PipelineStage
  pending?: boolean
  onOpenChange: (open: boolean) => void
  onSubmit: (values: StageFormValues) => void
}

const KINDS: StageKind[] = ['open', 'won', 'lost']

function StageForm({ stage, pending, onOpenChange, onSubmit }: Omit<StageDialogProps, 'open'>) {
  const { t } = useTranslation('pipelines', { keyPrefix: 'editor' })
  const { t: tp } = useTranslation('pipelines')
  const requirable = useRequirableFields()
  const [entry, setEntry] = useState<string[]>(stage?.entry_required_fields ?? [])
  const [exit, setExit] = useState<string[]>(stage?.exit_required_fields ?? [])
  const [name, setName] = useState(stage?.name ?? '')
  const [probability, setProbability] = useState(String(stage?.probability ?? 0))
  const [kind, setKind] = useState<StageKind>(stage?.kind ?? 'open')
  const [rottingDays, setRottingDays] = useState(String(stageRottingDays(stage) ?? ''))
  const [submitted, setSubmitted] = useState(false)
  const nameId = useId()
  const probabilityId = useId()
  const kindId = useId()
  const parsedProbability = Number(probability)
  const nameError = submitted && !name.trim()
  const probabilityError =
    submitted && (!Number.isInteger(parsedProbability) || parsedProbability < 0 || parsedProbability > 100)
  const kindLocked = Boolean(stage && (stage.deal_count ?? 0) > 0)
  const parsedRottingDays = kind === 'open' ? parseRottingDays(rottingDays) : null
  const rottingError = submitted && parsedRottingDays === undefined

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        setSubmitted(true)
        const valid =
          name.trim() &&
          Number.isInteger(parsedProbability) &&
          parsedProbability >= 0 &&
          parsedProbability <= 100 &&
          parsedRottingDays !== undefined
        if (valid) {
          onSubmit({
            name: name.trim(),
            probability: parsedProbability,
            kind,
            rotting_days: parsedRottingDays,
            entry_required_fields: entry,
            exit_required_fields: exit,
          })
        }
      }}
    >
      <DialogHeader>
        <DialogTitle>{stage ? t('settings.stages.editTitle') : t('settings.stages.newTitle')}</DialogTitle>
        <DialogDescription>{t('settings.stages.dialogDescription')}</DialogDescription>
      </DialogHeader>
      <div className="grid gap-2">
        <Label htmlFor={nameId}>{t('settings.stages.name')}</Label>
        <Input
          id={nameId}
          value={name}
          maxLength={140}
          aria-invalid={nameError || undefined}
          onChange={(event) => setName(event.target.value)}
        />
        {nameError && <p className="text-sm text-destructive">{t('settings.stages.nameRequired')}</p>}
      </div>
      <div className="grid gap-2">
        <Label htmlFor={probabilityId}>{t('settings.stages.probability')}</Label>
        <Input
          id={probabilityId}
          type="number"
          min={0}
          max={100}
          step={1}
          value={probability}
          aria-invalid={probabilityError || undefined}
          onChange={(event) => setProbability(event.target.value)}
        />
        {probabilityError && <p className="text-sm text-destructive">{t('settings.stages.probabilityInvalid')}</p>}
      </div>
      <div className="grid gap-2">
        <Label htmlFor={kindId}>{t('settings.stages.kind')}</Label>
        <select
          id={kindId}
          value={kind}
          disabled={kindLocked}
          onChange={(event) => setKind(event.target.value as StageKind)}
          className="h-9 w-full rounded-md border bg-background px-3 text-sm disabled:opacity-60"
        >
          {KINDS.map((value) => (
            <option key={value} value={value}>
              {t(`kind.${value}`)}
            </option>
          ))}
        </select>
        {kindLocked && <p className="text-xs text-muted-foreground">{t('settings.stages.kindLocked')}</p>}
      </div>
      <RottingDaysField
        value={kind === 'open' ? rottingDays : ''}
        disabled={kind !== 'open'}
        invalid={rottingError}
        onChange={setRottingDays}
      />
      <CheckboxList
        legend={tp('rules.entry')}
        description={tp('rules.entryDescription')}
        items={requirable}
        selected={entry}
        emptyText={tp('rules.noFields')}
        onChange={setEntry}
      />
      <CheckboxList
        legend={tp('rules.exit')}
        description={tp('rules.exitDescription')}
        items={requirable}
        selected={exit}
        emptyText={tp('rules.noFields')}
        onChange={setExit}
      />
      <DialogFooter>
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={pending}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" disabled={pending}>
          {t('actions.save')}
        </Button>
      </DialogFooter>
    </form>
  )
}

export function StageDialog({ open, onOpenChange, ...props }: StageDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto">
        {open && <StageForm {...props} onOpenChange={onOpenChange} />}
        {open && props.stage && <StageGuidanceEditor stageId={props.stage.id} />}
      </DialogContent>
    </Dialog>
  )
}
