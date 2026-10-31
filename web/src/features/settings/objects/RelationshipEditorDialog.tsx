import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { notifySuccess } from '@/components/feedback'
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
import {
  isCustomRecordType,
  notifyObjectsError,
  type Relationship,
  type RelationshipCardinality,
  type RelationshipOnDelete,
} from '@/features/custom-records'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { cn } from '@/lib/utils'
import { RELATION_KEY_PATTERN, objectsAdminApi } from './api'
import { relationKeyFromLabel } from './keys'
import { useRecordTypeLabels } from './labels'
import { Alert } from '@/components/ui/alert'
import { Field } from '@/components/ui/field'

const RELATION_KEY_MAX_LENGTH = 64
const LABEL_MAX_LENGTH = 100
const CARDINALITIES: RelationshipCardinality[] = ['one_to_many', 'one_to_one', 'many_to_many']
const ON_DELETE_OPTIONS: RelationshipOnDelete[] = ['restrict', 'unlink']
const DEFAULT_FROM_TYPE = 'company'

export interface RelationshipEditorDialogProps {
  relationship: Relationship | null
  recordType: string
  open: boolean
  onOpenChange: (open: boolean) => void
  onSaved: () => void
}

export function RelationshipEditorDialog({
  relationship,
  recordType,
  open,
  onOpenChange,
  onSaved,
}: RelationshipEditorDialogProps) {
  const { t } = useTranslation('objects')
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{relationship ? t('relationships.editTitle') : t('relationships.newTitle')}</DialogTitle>
          <DialogDescription>{t('relationships.dialogDescription')}</DialogDescription>
        </DialogHeader>
        {open && (
          <RelationshipForm
            key={relationship?.key ?? 'new'}
            relationship={relationship}
            recordType={recordType}
            onClose={() => onOpenChange(false)}
            onSaved={onSaved}
          />
        )}
      </DialogContent>
    </Dialog>
  )
}

interface RelationshipFormProps {
  relationship: Relationship | null
  recordType: string
  onClose: () => void
  onSaved: () => void
}

type FormErrors = Partial<Record<'key' | 'inverseKey' | 'fromLabel' | 'toLabel' | 'types' | 'form', string>>

function RelationshipForm({ relationship, recordType, onClose, onSaved }: RelationshipFormProps) {
  const { t } = useTranslation('objects')
  const formId = useId()
  const { label, options } = useRecordTypeLabels()
  const editing = relationship !== null
  const initialFrom = relationship?.from_type ?? (recordType === DEFAULT_FROM_TYPE ? 'contact' : DEFAULT_FROM_TYPE)
  const initialTo = relationship?.to_type ?? recordType
  const [fromType, setFromType] = useState(initialFrom)
  const [toType, setToType] = useState(initialTo)
  const [cardinality, setCardinality] = useState<RelationshipCardinality>(relationship?.cardinality ?? 'one_to_many')
  const [fromLabel, setFromLabel] = useState(relationship?.from_label ?? label(initialTo, true))
  const [toLabel, setToLabel] = useState(relationship?.to_label ?? label(initialFrom))
  const [key, setKey] = useState(relationship?.key ?? relationKeyFromLabel(label(initialTo, true), RELATION_KEY_MAX_LENGTH))
  const [inverseKey, setInverseKey] = useState(
    relationship?.inverse_key ?? relationKeyFromLabel(label(initialFrom), RELATION_KEY_MAX_LENGTH),
  )
  const [keysTouched, setKeysTouched] = useState({ key: editing, inverse: editing })
  const [labelsTouched, setLabelsTouched] = useState({ from: editing, to: editing })
  const [required, setRequired] = useState(relationship?.required ?? false)
  const [onDelete, setOnDelete] = useState<RelationshipOnDelete>(relationship?.on_delete ?? 'restrict')
  const [errors, setErrors] = useState<FormErrors>({})
  const [saving, setSaving] = useState(false)
  const typeOptions = options(editing ? undefined : recordType)
  const requiredAllowed = isCustomRecordType(toType)
  const fromName = label(fromType)
  const toName = label(toType)
  const fromPlural = label(fromType, true)
  const toPlural = label(toType, true)

  const applyFromLabel = (value: string, touched = true) => {
    setFromLabel(value)
    if (touched) setLabelsTouched((current) => ({ ...current, from: true }))
    if (!keysTouched.key) setKey(relationKeyFromLabel(value, RELATION_KEY_MAX_LENGTH))
    setErrors((current) => ({ ...current, fromLabel: undefined, key: undefined }))
  }

  const applyToLabel = (value: string, touched = true) => {
    setToLabel(value)
    if (touched) setLabelsTouched((current) => ({ ...current, to: true }))
    if (!keysTouched.inverse) setInverseKey(relationKeyFromLabel(value, RELATION_KEY_MAX_LENGTH))
    setErrors((current) => ({ ...current, toLabel: undefined, inverseKey: undefined }))
  }

  const changeFromType = (value: string) => {
    setFromType(value)
    setErrors((current) => ({ ...current, types: undefined }))
    if (!labelsTouched.to) applyToLabel(label(value), false)
  }

  const changeToType = (value: string) => {
    setToType(value)
    setErrors((current) => ({ ...current, types: undefined }))
    if (!isCustomRecordType(value)) setRequired(false)
    if (!labelsTouched.from) applyFromLabel(label(value, true), false)
  }

  const validate = (): FormErrors => {
    const next: FormErrors = {}
    if (!fromLabel.trim()) next.fromLabel = t('relationships.errors.labelRequired')
    if (!toLabel.trim()) next.toLabel = t('relationships.errors.labelRequired')
    if (!editing) {
      if (!RELATION_KEY_PATTERN.test(key)) next.key = t('relationships.errors.keyInvalid')
      if (!RELATION_KEY_PATTERN.test(inverseKey)) next.inverseKey = t('relationships.errors.keyInvalid')
      if (fromType === toType && key === inverseKey && !next.key) next.inverseKey = t('relationships.errors.sameKeys')
    }
    return next
  }

  const submit = async () => {
    const found = validate()
    if (Object.values(found).some(Boolean)) {
      setErrors(found)
      return
    }
    setSaving(true)
    try {
      if (relationship) {
        await objectsAdminApi.updateRelationship(
          relationship.key,
          {
            from_label: fromLabel.trim(),
            to_label: toLabel.trim(),
            required: requiredAllowed && required,
            on_delete: onDelete,
          },
          relationship.updated_at,
        )
      } else {
        await objectsAdminApi.createRelationship({
          key,
          inverse_key: inverseKey,
          from_type: fromType,
          to_type: toType,
          cardinality,
          from_label: fromLabel.trim(),
          to_label: toLabel.trim(),
          required: requiredAllowed && required,
          on_delete: onDelete,
        })
      }
      notifySuccess(t(editing ? 'relationships.updated' : 'relationships.created'))
      onSaved()
      onClose()
    } catch (error) {
      if (isApiError(error) && error.code === 'version_conflict') setErrors({ form: t('errors.versionConflict') })
      else if (isApiError(error) && error.code === 'validation_error') {
        setErrors({ form: t('relationships.errors.serverValidation') })
      } else notifyObjectsError(error)
    } finally {
      setSaving(false)
    }
  }

  const errorText = (name: keyof FormErrors) =>
    errors[name] ? (
      <SharedFieldError id={`${formId}-${name}-error`}>
        {errors[name]}
      </SharedFieldError>
    ) : null
  const describedBy = (name: keyof FormErrors) => (errors[name] ? `${formId}-${name}-error` : undefined)

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
        {errors.form && (
          <Alert className="text-destructive">
            {errors.form}
          </Alert>
        )}
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label={t('relationships.from')}>
            {(control) => <NativeSelect
              {...control}
              value={fromType}
              disabled={editing}
              aria-describedby={describedBy('types')}
              onChange={(event) => changeFromType(event.target.value)}
            >
              {!typeOptions.some((option) => option.value === fromType) && (
                <option value={fromType}>{fromName}</option>
              )}
              {typeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.singular}
                </option>
              ))}
            </NativeSelect>}
          </Field>
          <Field label={t('relationships.to')}>
            {(control) => <NativeSelect
              {...control}
              value={toType}
              disabled={editing}
              aria-describedby={describedBy('types')}
              onChange={(event) => changeToType(event.target.value)}
            >
              {!typeOptions.some((option) => option.value === toType) && <option value={toType}>{toName}</option>}
              {typeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.singular}
                </option>
              ))}
            </NativeSelect>}
          </Field>
        </div>
        {errorText('types')}
        <fieldset className="grid gap-2" disabled={editing}>
          <legend className="text-sm font-medium">{t('relationships.cardinality')}</legend>
          <div className="grid gap-2">
            {CARDINALITIES.map((value) => (
              <label
                key={value}
                className={cn(
                  'flex cursor-pointer items-start gap-2 rounded-md border p-3 text-sm',
                  cardinality === value && 'border-primary bg-primary/5',
                  editing && 'cursor-not-allowed opacity-70',
                )}
              >
                <input
                  type="radio"
                  name={`${formId}-cardinality`}
                  value={value}
                  checked={cardinality === value}
                  className="mt-1"
                  onChange={() => setCardinality(value)}
                />
                <span className="grid gap-0.5">
                  <span className="font-medium">{t(`relationships.cardinalities.${value}.title`)}</span>
                  <span className="text-xs text-muted-foreground">
                    {t(`relationships.cardinalities.${value}.description`, {
                      from: fromName,
                      to: toName,
                      fromPlural,
                      toPlural,
                    })}
                  </span>
                </span>
              </label>
            ))}
          </div>
        </fieldset>
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label={t('relationships.fromLabel', { type: fromName })}>
            {(control) => <>
              <Input placeholder={t('relationships.editor.fromLabelPlaceholder')}
              {...control}
              value={fromLabel}
              maxLength={LABEL_MAX_LENGTH}
              aria-invalid={errors.fromLabel ? true : undefined}
              aria-describedby={describedBy('fromLabel')}
              onChange={(event) => applyFromLabel(event.target.value)}
            />
            {errorText('fromLabel')}
            </>}
          </Field>
          <Field label={t('relationships.toLabel', { type: toName })}>
            {(control) => <>
              <Input placeholder={t('relationships.editor.toLabelPlaceholder')}
              {...control}
              value={toLabel}
              maxLength={LABEL_MAX_LENGTH}
              aria-invalid={errors.toLabel ? true : undefined}
              aria-describedby={describedBy('toLabel')}
              onChange={(event) => applyToLabel(event.target.value)}
            />
            {errorText('toLabel')}
            </>}
          </Field>
          <Field label={t('relationships.key', { type: fromName })}>
            {(control) => <>
              <Input placeholder={t('relationships.editor.keyPlaceholder')}
              {...control}
              value={key}
              maxLength={RELATION_KEY_MAX_LENGTH}
              disabled={editing}
              className="font-mono"
              aria-invalid={errors.key ? true : undefined}
              aria-describedby={describedBy('key')}
              onChange={(event) => {
                setKey(event.target.value.toLowerCase())
                setKeysTouched((current) => ({ ...current, key: true }))
                setErrors((current) => ({ ...current, key: undefined }))
              }}
            />
            {errorText('key')}
            </>}
          </Field>
          <Field label={t('relationships.inverseKey', { type: toName })}>
            {(control) => <>
              <Input placeholder={t('relationships.editor.inverseKeyPlaceholder')}
              {...control}
              value={inverseKey}
              maxLength={RELATION_KEY_MAX_LENGTH}
              disabled={editing}
              className="font-mono"
              aria-invalid={errors.inverseKey ? true : undefined}
              aria-describedby={describedBy('inverseKey')}
              onChange={(event) => {
                setInverseKey(event.target.value.toLowerCase())
                setKeysTouched((current) => ({ ...current, inverse: true }))
                setErrors((current) => ({ ...current, inverseKey: undefined }))
              }}
            />
            {errorText('inverseKey')}
            </>}
          </Field>
        </div>
        <p className="text-xs text-muted-foreground">{t('relationships.keysHint')}</p>
        <label className={cn('flex items-start gap-2 text-sm', !requiredAllowed && 'opacity-60')}>
          <Checkbox
            className="mt-0.5"
            checked={requiredAllowed && required}
            disabled={!requiredAllowed}
            onCheckedChange={(checked) => setRequired(checked === true)}
          />
          <span className="grid gap-0.5">
            <span className="font-medium">{t('relationships.required')}</span>
            <span className="text-xs text-muted-foreground">
              {requiredAllowed
                ? t('relationships.requiredHint', { from: fromName, to: toName })
                : t('relationships.requiredUnavailable')}
            </span>
          </span>
        </label>
        <Field label={t('relationships.onDelete', { type: fromName })}>
          {(control) => <NativeSelect
            {...control}
            value={onDelete}
            onChange={(event) => setOnDelete(event.target.value as RelationshipOnDelete)}
          >
            {ON_DELETE_OPTIONS.map((value) => (
              <option key={value} value={value}>
                {t(`relationships.onDeleteOptions.${value}`, { from: fromName, toPlural })}
              </option>
            ))}
          </NativeSelect>}
        </Field>
      </form>
      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={saving}>
          {t('editor.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={saving} aria-busy={saving}>
          {saving ? t('editor.saving') : editing ? t('editor.save') : t('editor.create')}
        </Button>
      </DialogFooter>
    </>
  )
}
