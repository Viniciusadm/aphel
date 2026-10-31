import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-react'
import { useId, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
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
import { Textarea } from '@/components/ui/textarea'
import { useFieldSchema } from '@/features/records/api'
import { FilterBuilder, emptyGroup, serializeFilter } from '@/features/records/filters'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import type { FieldSchema } from '@/features/records/types'
import { scoringApi } from './api'
import { useScoringRecordTypeLabel, useScoringRecordTypes } from './labels'
import {
  DESCRIPTION_MAX_LENGTH,
  LABEL_MAX_LENGTH,
  MAX_CRITERIA,
  NAME_MAX_LENGTH,
  createRequest,
  draftFromModel,
  emptyModelDraft,
  newCriterion,
  serverErrors,
  updateRequest,
  validateDraft,
  type CriterionDraft,
  type FormError,
  type FormErrors,
  type ModelDraft,
} from './scoring-form'
import { SCORING_SIGNALS, type ScoringCriterionKind, type ScoringModel, type ScoringSignal } from './types'

const LOCAL_ERRORS: FormError[] = ['required', 'invalidKey', 'points', 'cap', 'decay', 'bands', 'filter', 'integer']

interface ScoringModelDialogProps {
  open: boolean
  model: ScoringModel | null
  hasPrimary: (recordType: string) => boolean
  onOpenChange: (open: boolean) => void
  onSaved: (created: boolean) => void
}

export function ScoringModelDialog({ open, onOpenChange, ...props }: ScoringModelDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="2xl">
        {open && <ScoringModelForm key={props.model?.id ?? 'new'} onOpenChange={onOpenChange} {...props} />}
      </DialogContent>
    </Dialog>
  )
}

type FormProps = Omit<ScoringModelDialogProps, 'open'>

function ScoringModelForm({ model, hasPrimary, onOpenChange, onSaved }: FormProps) {
  const { t } = useTranslation('scoring')
  const creating = model === null
  const [draft, setDraft] = useState<ModelDraft>(() => {
    if (model) return draftFromModel(model)
    const initial = emptyModelDraft('contact')
    return { ...initial, isPrimary: !hasPrimary(initial.recordType) }
  })
  const [errors, setErrors] = useState<FormErrors>({})
  const [generalError, setGeneralError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const schema = useFieldSchema(draft.recordType)
  const recordTypes = useScoringRecordTypes(draft.recordType)
  const recordTypeLabel = useScoringRecordTypeLabel()
  const ids = {
    name: useId(),
    key: useId(),
    description: useId(),
    recordType: useId(),
    primary: useId(),
    active: useId(),
    warm: useId(),
    hot: useId(),
  }

  const patch = (next: Partial<ModelDraft>) => setDraft((current) => ({ ...current, ...next }))

  const patchCriterion = (uid: string, next: Partial<CriterionDraft>) =>
    setDraft((current) => ({
      ...current,
      criteria: current.criteria.map((criterion) => (criterion.uid === uid ? { ...criterion, ...next } : criterion)),
    }))

  const moveCriterion = (index: number, offset: number) =>
    setDraft((current) => {
      const target = index + offset
      if (target < 0 || target >= current.criteria.length) return current
      const criteria = [...current.criteria]
      ;[criteria[index], criteria[target]] = [criteria[target], criteria[index]]
      return { ...current, criteria }
    })

  const errorText = (value: string | undefined): string | null => {
    if (!value) return null
    return (LOCAL_ERRORS as string[]).includes(value) ? t(`dialog.errors.${value as FormError}`) : value
  }

  const fieldError = (field: string) => {
    const text = errorText(errors[field])
    return text ? <p className="text-xs break-words text-destructive">{text}</p> : null
  }

  const changeRecordType = (recordType: string) =>
    patch({
      recordType,
      isPrimary: !hasPrimary(recordType),
      criteria: draft.criteria.map((criterion) => ({ ...criterion, filter: emptyGroup() })),
    })

  const submit = async () => {
    const local = validateDraft(draft, creating)
    if (schema.data) {
      draft.criteria.forEach((criterion, index) => {
        if (criterion.kind === 'profile' && !serializeFilter(criterion.filter, schema.data))
          local[`criteria.${index}.filter`] = 'filter'
      })
    }
    setErrors(local)
    setGeneralError(null)
    if (Object.keys(local).length > 0) return
    setSaving(true)
    try {
      if (model) await scoringApi.update(model.id, updateRequest(draft, schema.data), model.updated_at)
      else await scoringApi.create(createRequest(draft, schema.data))
      onSaved(creating)
    } catch (error) {
      if (isApiError(error) && error.status === 409) {
        setGeneralError(t('toasts.conflict'))
      } else {
        const server = isApiError(error) ? serverErrors(error.details) : {}
        setErrors(server)
        if (Object.keys(server).length === 0) setGeneralError(t('toasts.error'))
      }
    } finally {
      setSaving(false)
    }
  }

  const knownFields = new Set(['name', 'key', 'description', 'record_type', 'warm_from', 'hot_from'])
  const otherErrors = Object.entries(errors).filter(
    ([field]) => !knownFields.has(field) && !field.startsWith('criteria.'),
  )

  return (
    <form
      className="grid min-w-0 gap-4"
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
    >
      <DialogHeader>
        <DialogTitle>{creating ? t('dialog.newTitle') : t('dialog.editTitle')}</DialogTitle>
        <DialogDescription>{t('dialog.description')}</DialogDescription>
      </DialogHeader>

      <div className="grid min-w-0 gap-4 sm:grid-cols-2">
        <div className="grid min-w-0 content-start gap-2">
          <Label htmlFor={ids.name}>{t('dialog.name')}</Label>
          <Input placeholder={t('placeholders.name')}
            id={ids.name}
            value={draft.name}
            maxLength={NAME_MAX_LENGTH}
            aria-invalid={errors.name ? true : undefined}
            onChange={(event) => patch({ name: event.target.value })}
          />
          {fieldError('name')}
        </div>
        <div className="grid min-w-0 content-start gap-2">
          <Label htmlFor={ids.key}>{t('dialog.key')}</Label>
          <Input placeholder={t('placeholders.key')}
            id={ids.key}
            value={draft.key}
            disabled={!creating}
            maxLength={64}
            className="font-mono"
            aria-describedby={`${ids.key}-hint`}
            aria-invalid={errors.key ? true : undefined}
            onChange={(event) => patch({ key: event.target.value })}
          />
          <p id={`${ids.key}-hint`} className="text-xs text-muted-foreground">
            {t('dialog.keyHint')}
          </p>
          {fieldError('key')}
        </div>
      </div>

      <div className="grid min-w-0 gap-2">
        <Label htmlFor={ids.description}>{t('dialog.descriptionLabel')}</Label>
        <Textarea placeholder={t('placeholders.description')}
          id={ids.description}
          value={draft.description}
          rows={2}
          maxLength={DESCRIPTION_MAX_LENGTH}
          onChange={(event) => patch({ description: event.target.value })}
        />
        {fieldError('description')}
      </div>

      <div className="grid min-w-0 gap-4 sm:grid-cols-2">
        <div className="grid min-w-0 content-start gap-2">
          <Label htmlFor={ids.recordType}>{t('dialog.recordType')}</Label>
          <NativeSelect
            id={ids.recordType}
            value={draft.recordType}
            disabled={!creating}
            onChange={(event) => changeRecordType(event.target.value)}
          >
            {recordTypes.map((recordType) => (
              <option key={recordType} value={recordType}>
                {recordTypeLabel(recordType)}
              </option>
            ))}
          </NativeSelect>
          {fieldError('record_type')}
        </div>
        <div className="grid min-w-0 content-start gap-2 pt-1 sm:pt-7">
          <label htmlFor={ids.primary} className="flex items-center gap-2 text-sm">
            <Checkbox
              id={ids.primary}
              checked={draft.isPrimary}
              onCheckedChange={(value) => patch({ isPrimary: value === true })}
            />
            {t('dialog.primary')}
          </label>
          <label htmlFor={ids.active} className="flex items-center gap-2 text-sm">
            <Checkbox
              id={ids.active}
              checked={draft.isActive}
              onCheckedChange={(value) => patch({ isActive: value === true })}
            />
            {t('dialog.active')}
          </label>
        </div>
      </div>

      <fieldset className="grid min-w-0 gap-3 rounded-md border p-3">
        <legend className="px-1 text-sm font-medium">{t('dialog.bandsTitle')}</legend>
        <div className="grid min-w-0 gap-4 sm:grid-cols-2">
          <div className="grid min-w-0 content-start gap-2">
            <Label htmlFor={ids.warm}>{t('dialog.warmFrom')}</Label>
            <Input placeholder={t('placeholders.warm')}
              id={ids.warm}
              inputMode="numeric"
              value={draft.warmFrom}
              aria-invalid={errors.warm_from ? true : undefined}
              onChange={(event) => patch({ warmFrom: event.target.value })}
            />
            {fieldError('warm_from')}
          </div>
          <div className="grid min-w-0 content-start gap-2">
            <Label htmlFor={ids.hot}>{t('dialog.hotFrom')}</Label>
            <Input placeholder={t('placeholders.hot')}
              id={ids.hot}
              inputMode="numeric"
              value={draft.hotFrom}
              aria-invalid={errors.hot_from ? true : undefined}
              onChange={(event) => patch({ hotFrom: event.target.value })}
            />
            {fieldError('hot_from')}
          </div>
        </div>
        <p className="text-xs text-muted-foreground">{t('dialog.bandsHint')}</p>
      </fieldset>

      <section className="grid min-w-0 gap-3" aria-labelledby={`${ids.name}-criteria`}>
        <div className="grid gap-1">
          <h3 id={`${ids.name}-criteria`} className="text-sm font-medium">
            {t('dialog.criteriaTitle')}
          </h3>
          <p className="text-xs text-muted-foreground">{t('dialog.criteriaHint')}</p>
        </div>
        {draft.criteria.length === 0 && <p className="text-sm text-muted-foreground">{t('dialog.noCriteria')}</p>}
        <ol className="grid min-w-0 gap-3">
          {draft.criteria.map((criterion, index) => (
            <CriterionEditor
              key={criterion.uid}
              criterion={criterion}
              index={index}
              total={draft.criteria.length}
              schema={schema.data}
              error={fieldError}
              onChange={(next) => patchCriterion(criterion.uid, next)}
              onMove={(offset) => moveCriterion(index, offset)}
              onRemove={() =>
                patch({ criteria: draft.criteria.filter((item) => item.uid !== criterion.uid) })
              }
            />
          ))}
        </ol>
        <div className="flex flex-wrap gap-2">
          {(['profile', 'behavior'] as ScoringCriterionKind[]).map((kind) => (
            <Button
              key={kind}
              type="button"
              variant="outline"
              size="sm"
              disabled={draft.criteria.length >= MAX_CRITERIA}
              onClick={() => patch({ criteria: [...draft.criteria, newCriterion(kind)] })}
            >
              <Plus aria-hidden="true" />
              {kind === 'profile' ? t('dialog.addProfile') : t('dialog.addBehavior')}
            </Button>
          ))}
        </div>
      </section>

      {(generalError || otherErrors.length > 0) && (
        <div className="grid gap-1" role="alert">
          {generalError && <p className="text-sm text-destructive">{generalError}</p>}
          {otherErrors.map(([field, message]) => (
            <p key={field} className="text-sm break-words text-destructive">
              {errorText(message)}
            </p>
          ))}
        </div>
      )}

      <DialogFooter>
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
          {t('dialog.cancel')}
        </Button>
        <Button type="submit" disabled={saving}>
          {saving ? t('dialog.saving') : t('dialog.save')}
        </Button>
      </DialogFooter>
    </form>
  )
}

interface CriterionEditorProps {
  criterion: CriterionDraft
  index: number
  total: number
  schema: FieldSchema | undefined
  error: (field: string) => ReactNode
  onChange: (next: Partial<CriterionDraft>) => void
  onMove: (offset: number) => void
  onRemove: () => void
}

function CriterionEditor({ criterion, index, total, schema, error, onChange, onMove, onRemove }: CriterionEditorProps) {
  const { t } = useTranslation('scoring')
  const id = useId()
  const path = (field: string) => `criteria.${index}.${field}`
  const name = criterion.label || t(`kinds.${criterion.kind}`)

  return (
    <li className="grid min-w-0 gap-3 rounded-md border bg-card p-3">
      <div className="flex min-w-0 flex-wrap items-center justify-between gap-2">
        <span className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
          {t(`kinds.${criterion.kind}`)}
        </span>
        <div className="flex items-center gap-1">
          <Button
            type="button"
            size="icon"
            variant="ghost"
            disabled={index === 0}
            aria-label={t('dialog.moveUp', { label: name })}
            onClick={() => onMove(-1)}
          >
            <ArrowUp aria-hidden="true" />
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            disabled={index === total - 1}
            aria-label={t('dialog.moveDown', { label: name })}
            onClick={() => onMove(1)}
          >
            <ArrowDown aria-hidden="true" />
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className="text-destructive"
            aria-label={t('dialog.removeCriterion', { label: name })}
            onClick={onRemove}
          >
            <Trash2 aria-hidden="true" />
          </Button>
        </div>
      </div>
      <div className="grid min-w-0 gap-3 sm:grid-cols-[minmax(0,2fr)_minmax(0,1fr)_minmax(0,1fr)]">
        <div className="grid min-w-0 content-start gap-1.5">
          <Label htmlFor={`${id}-label`}>{t('dialog.criterionLabel')}</Label>
          <Input placeholder={t('placeholders.criterionLabel')}
            id={`${id}-label`}
            value={criterion.label}
            maxLength={LABEL_MAX_LENGTH}
            aria-invalid={error(path('label')) ? true : undefined}
            onChange={(event) => onChange({ label: event.target.value })}
          />
          {error(path('label'))}
        </div>
        <div className="grid min-w-0 content-start gap-1.5">
          <Label htmlFor={`${id}-key`}>{t('dialog.criterionKey')}</Label>
          <Input placeholder={t('placeholders.criterionKey')}
            id={`${id}-key`}
            value={criterion.key}
            maxLength={64}
            className="font-mono"
            onChange={(event) => onChange({ key: event.target.value })}
          />
          {error(path('key'))}
        </div>
        <div className="grid min-w-0 content-start gap-1.5">
          <Label htmlFor={`${id}-points`}>{t('dialog.points')}</Label>
          <Input placeholder={t('placeholders.points')}
            id={`${id}-points`}
            inputMode="numeric"
            value={criterion.points}
            aria-describedby={`${id}-points-hint`}
            onChange={(event) => onChange({ points: event.target.value })}
          />
          <p id={`${id}-points-hint`} className="text-xs text-muted-foreground">
            {t('dialog.pointsHint')}
          </p>
          {error(path('points'))}
        </div>
      </div>
      {criterion.kind === 'profile' ? (
        <div className="grid min-w-0 gap-1.5">
          <span className="text-sm font-medium">{t('dialog.filter')}</span>
          {schema && (
            <FilterBuilder schema={schema} value={criterion.filter} onChange={(filter) => onChange({ filter })} />
          )}
          {error(path('filter'))}
        </div>
      ) : (
        <div className="grid min-w-0 gap-3 sm:grid-cols-3">
          <div className="grid min-w-0 content-start gap-1.5">
            <Label htmlFor={`${id}-signal`}>{t('dialog.signal')}</Label>
            <NativeSelect
              id={`${id}-signal`}
              value={criterion.signal}
              onChange={(event) => onChange({ signal: event.target.value as ScoringSignal })}
            >
              {SCORING_SIGNALS.map((signal) => (
                <option key={signal} value={signal}>
                  {t(`signals.${signal}`)}
                </option>
              ))}
            </NativeSelect>
            {error(path('signal'))}
          </div>
          <div className="grid min-w-0 content-start gap-1.5">
            <Label htmlFor={`${id}-cap`}>{t('dialog.maxPoints')}</Label>
            <Input placeholder={t('placeholders.maxPoints')}
              id={`${id}-cap`}
              inputMode="numeric"
              value={criterion.maxPoints}
              aria-describedby={`${id}-cap-hint`}
              onChange={(event) => onChange({ maxPoints: event.target.value })}
            />
            <p id={`${id}-cap-hint`} className="text-xs text-muted-foreground">
              {t('dialog.maxPointsHint')}
            </p>
            {error(path('max_points'))}
          </div>
          <div className="grid min-w-0 content-start gap-1.5">
            <Label htmlFor={`${id}-decay`}>{t('dialog.decayDays')}</Label>
            <Input placeholder={t('placeholders.decayDays')}
              id={`${id}-decay`}
              inputMode="numeric"
              value={criterion.decayDays}
              aria-describedby={`${id}-decay-hint`}
              onChange={(event) => onChange({ decayDays: event.target.value })}
            />
            <p id={`${id}-decay-hint`} className="text-xs text-muted-foreground">
              {t('dialog.decayHint')}
            </p>
            {error(path('decay_days'))}
          </div>
        </div>
      )}
    </li>
  )
}
