import { DialogForm } from '@/components/ui/dialog-form'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import type { PipelineStage, StageKind } from './types'
import { useFieldSchema } from '@/features/records/api'
import { activeFields } from '@/features/records/fields'
import { CheckboxList, type CheckboxListItem } from './CheckboxList'
import { StageGuidanceEditor } from './guidance/StageGuidanceEditor'
import { REQUIRABLE_SYSTEM_FIELDS } from './requirements'
import { DEAL_RECORD_TYPE } from './resource'
import { parseRottingDays, stageRottingDays } from './rotting/lib'
import { RottingDaysField } from './rotting/RottingDaysField'
import { NativeSelect } from '@/components/ui/native-select'
import { Field } from '@/components/ui/field'

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
  const parsedProbability = Number(probability)
  const nameError = submitted && !name.trim()
  const probabilityError =
    submitted && (!Number.isInteger(parsedProbability) || parsedProbability < 0 || parsedProbability > 100)
  const kindLocked = Boolean(stage && (stage.deal_count ?? 0) > 0)
  const parsedRottingDays = kind === 'open' ? parseRottingDays(rottingDays) : null
  const rottingError = submitted && parsedRottingDays === undefined

  return (
    <DialogForm
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
      title={stage ? t('settings.stages.editTitle') : t('settings.stages.newTitle')}
      description={t('settings.stages.dialogDescription')}
      cancelLabel={t('actions.cancel')}
      submitLabel={t('actions.save')}
      onCancel={() => onOpenChange(false)}
      pending={pending}
      submitDisabled={pending}
    >
      <Field label={t('settings.stages.name')} error={nameError ? t('settings.stages.nameRequired') : undefined}>
        {(control) => <>
          <Input placeholder={t('placeholders.stageName')}
          {...control}
          value={name}
          maxLength={140}
          aria-invalid={nameError || undefined}
          onChange={(event) => setName(event.target.value)}
        />
        </>}
      </Field>
      <Field label={t('settings.stages.probability')} error={probabilityError ? t('settings.stages.probabilityInvalid') : undefined}>
        {(control) => <>
          <Input placeholder={t('placeholders.probability')}
          {...control}
          type="number"
          min={0}
          max={100}
          step={1}
          value={probability}
          aria-invalid={probabilityError || undefined}
          onChange={(event) => setProbability(event.target.value)}
        />
        </>}
      </Field>
      <Field label={t('settings.stages.kind')}>
        {(control) => <>
          <NativeSelect
          {...control}
          value={kind}
          disabled={kindLocked}
          onChange={(event) => setKind(event.target.value as StageKind)}
          className="w-full"
        >
          {KINDS.map((value) => (
            <option key={value} value={value}>
              {t(`kind.${value}`)}
            </option>
          ))}
        </NativeSelect>
        {kindLocked && <p className="text-xs text-muted-foreground">{t('settings.stages.kindLocked')}</p>}
        </>}
      </Field>
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
    </DialogForm>
  )
}

export function StageDialog({ open, onOpenChange, ...props }: StageDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        {open && <StageForm {...props} onOpenChange={onOpenChange} />}
        {open && props.stage && <StageGuidanceEditor stageId={props.stage.id} />}
      </DialogContent>
    </Dialog>
  )
}
