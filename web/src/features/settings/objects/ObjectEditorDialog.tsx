import { useId, useMemo, useState } from 'react'
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
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import {
  DEFAULT_OBJECT_COLOR,
  DEFAULT_OBJECT_ICON,
  OBJECT_COLORS,
  OBJECT_ICON_NAMES,
  notifyObjectsError,
  objectColor,
  ObjectIcon,
} from '@/features/custom-records'
import { useFieldSchema } from '@/features/records/api'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { cn } from '@/lib/utils'
import { OBJECT_KEY_MAX_LENGTH, OBJECT_KEY_PATTERN, objectsAdminApi, type CustomObjectAdmin } from './api'
import { objectKeyFromLabel } from './keys'

const LABEL_MAX_LENGTH = 100
const TITLE_FIELD_TYPES = ['text', 'email', 'phone', 'url']

export interface ObjectEditorDialogProps {
  object: CustomObjectAdmin | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onSaved: (object: CustomObjectAdmin) => void
}

export function ObjectEditorDialog({ object, open, onOpenChange, onSaved }: ObjectEditorDialogProps) {
  const { t } = useTranslation('objects')
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{object ? t('editor.editTitle', { label: object.label_plural }) : t('editor.newTitle')}</DialogTitle>
          <DialogDescription>{t('editor.description')}</DialogDescription>
        </DialogHeader>
        {open && (
          <ObjectForm key={object?.key ?? 'new'} object={object} onClose={() => onOpenChange(false)} onSaved={onSaved} />
        )}
      </DialogContent>
    </Dialog>
  )
}

interface ObjectFormProps {
  object: CustomObjectAdmin | null
  onClose: () => void
  onSaved: (object: CustomObjectAdmin) => void
}

type FormErrors = Partial<Record<'labelSingular' | 'labelPlural' | 'key' | 'form', string>>

function ObjectForm({ object, onClose, onSaved }: ObjectFormProps) {
  const { t } = useTranslation('objects')
  const formId = useId()
  const editing = object !== null
  const [labelSingular, setLabelSingular] = useState(object?.label_singular ?? '')
  const [labelPlural, setLabelPlural] = useState(object?.label_plural ?? '')
  const [key, setKey] = useState(object?.key ?? '')
  const [keyTouched, setKeyTouched] = useState(editing)
  const [description, setDescription] = useState(object?.description ?? '')
  const [icon, setIcon] = useState(object?.icon ?? DEFAULT_OBJECT_ICON)
  const [color, setColor] = useState(objectColor(object?.color) ?? DEFAULT_OBJECT_COLOR)
  const [ownerEnabled, setOwnerEnabled] = useState(object?.owner_enabled ?? true)
  const [isActive, setIsActive] = useState(object?.is_active ?? true)
  const [titleField, setTitleField] = useState(object?.title_field_key ?? 'name')
  const [errors, setErrors] = useState<FormErrors>({})
  const [saving, setSaving] = useState(false)

  const changeSingular = (value: string) => {
    setLabelSingular(value)
    setErrors((current) => ({ ...current, labelSingular: undefined }))
    if (!keyTouched) setKey(objectKeyFromLabel(value, OBJECT_KEY_MAX_LENGTH))
  }

  const validate = (): FormErrors => {
    const next: FormErrors = {}
    if (!labelSingular.trim()) next.labelSingular = t('editor.errors.labelRequired')
    if (!labelPlural.trim()) next.labelPlural = t('editor.errors.labelRequired')
    if (!editing && (!OBJECT_KEY_PATTERN.test(key) || key.length > OBJECT_KEY_MAX_LENGTH)) {
      next.key = t('editor.errors.keyInvalid', { max: OBJECT_KEY_MAX_LENGTH })
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
    const common = {
      label_singular: labelSingular.trim(),
      label_plural: labelPlural.trim(),
      description: description.trim() || null,
      icon,
      color,
      owner_enabled: ownerEnabled,
    }
    try {
      const saved = object
        ? await objectsAdminApi.update(
            object.key,
            { ...common, is_active: isActive, title_field_key: titleField || undefined },
            object.updated_at,
          )
        : await objectsAdminApi.create({ ...common, key })
      notifySuccess(t(editing ? 'editor.updated' : 'editor.created', { label: saved.label_plural }))
      onSaved(saved)
      onClose()
    } catch (error) {
      if (isApiError(error) && error.code === 'custom_object_key_taken') setErrors({ key: t('errors.customObjectKeyTaken') })
      else if (isApiError(error) && error.code === 'version_conflict') setErrors({ form: t('errors.versionConflict') })
      else notifyObjectsError(error)
    } finally {
      setSaving(false)
    }
  }

  const fieldError = (name: keyof FormErrors) =>
    errors[name] ? (
      <p id={`${formId}-${name}-error`} className="text-xs text-destructive">
        {errors[name]}
      </p>
    ) : null
  const describedBy = (name: keyof FormErrors, hint?: string) =>
    [errors[name] ? `${formId}-${name}-error` : null, hint ?? null].filter(Boolean).join(' ') || undefined

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
          <p role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
            {errors.form}
          </p>
        )}
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="grid gap-1.5">
            <Label htmlFor={`${formId}-singular`}>{t('editor.labelSingular')}</Label>
            <Input
              id={`${formId}-singular`}
              value={labelSingular}
              maxLength={LABEL_MAX_LENGTH}
              placeholder={t('editor.labelSingularPlaceholder')}
              autoFocus
              aria-invalid={errors.labelSingular ? true : undefined}
              aria-describedby={describedBy('labelSingular')}
              onChange={(event) => changeSingular(event.target.value)}
            />
            {fieldError('labelSingular')}
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor={`${formId}-plural`}>{t('editor.labelPlural')}</Label>
            <Input
              id={`${formId}-plural`}
              value={labelPlural}
              maxLength={LABEL_MAX_LENGTH}
              placeholder={t('editor.labelPluralPlaceholder')}
              aria-invalid={errors.labelPlural ? true : undefined}
              aria-describedby={describedBy('labelPlural')}
              onChange={(event) => {
                setLabelPlural(event.target.value)
                setErrors((current) => ({ ...current, labelPlural: undefined }))
              }}
            />
            {fieldError('labelPlural')}
          </div>
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${formId}-key`}>{t('editor.key')}</Label>
          <Input
            id={`${formId}-key`}
            value={key}
            maxLength={OBJECT_KEY_MAX_LENGTH}
            disabled={editing}
            className="font-mono"
            aria-invalid={errors.key ? true : undefined}
            aria-describedby={describedBy('key', `${formId}-key-hint`)}
            onChange={(event) => {
              setKey(event.target.value.toLowerCase())
              setKeyTouched(true)
              setErrors((current) => ({ ...current, key: undefined }))
            }}
          />
          <p id={`${formId}-key-hint`} className="text-xs text-muted-foreground">
            {editing ? t('editor.keyLocked') : t('editor.keyHint', { recordType: `co_${key || '…'}` })}
          </p>
          {fieldError('key')}
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${formId}-description`}>{t('editor.descriptionLabel')}</Label>
          <Textarea
            id={`${formId}-description`}
            value={description}
            rows={2}
            onChange={(event) => setDescription(event.target.value)}
          />
        </div>
        <fieldset className="grid gap-2">
          <legend className="text-sm font-medium">{t('editor.icon')}</legend>
          <div className="flex flex-wrap gap-1.5">
            {OBJECT_ICON_NAMES.map((name) => {
              const selected = icon === name
              return (
                <button
                  key={name}
                  type="button"
                  aria-pressed={selected}
                  aria-label={t('editor.iconOption', { name })}
                  title={name}
                  className={cn(
                    'flex size-9 items-center justify-center rounded-md border transition-colors outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring',
                    selected && 'border-primary bg-primary/10',
                  )}
                  onClick={() => setIcon(name)}
                >
                  <ObjectIcon icon={name} className="size-4" style={{ color: selected ? color : undefined }} aria-hidden />
                </button>
              )
            })}
          </div>
        </fieldset>
        <fieldset className="grid gap-2">
          <legend className="text-sm font-medium">{t('editor.color')}</legend>
          <div className="flex flex-wrap items-center gap-1.5">
            {OBJECT_COLORS.map((swatch) => (
              <button
                key={swatch}
                type="button"
                aria-pressed={color === swatch}
                aria-label={t('editor.colorOption', { color: swatch })}
                className={cn(
                  'size-7 rounded-full border-2 border-transparent outline-none focus-visible:ring-2 focus-visible:ring-ring',
                  color === swatch && 'border-foreground',
                )}
                style={{ backgroundColor: swatch }}
                onClick={() => setColor(swatch)}
              />
            ))}
            <input
              type="color"
              aria-label={t('editor.customColor')}
              className="h-8 w-10 cursor-pointer rounded-md border border-input bg-background p-1"
              value={color}
              onChange={(event) => setColor(event.target.value)}
            />
          </div>
        </fieldset>
        {object && (
          <TitleFieldSelect
            id={`${formId}-title`}
            recordType={object.record_type}
            currentKey={object.title_field_key}
            value={titleField}
            onChange={setTitleField}
          />
        )}
        <div className="grid gap-3">
          <label className="flex items-start gap-2 text-sm">
            <Checkbox
              className="mt-0.5"
              checked={ownerEnabled}
              onCheckedChange={(checked) => setOwnerEnabled(checked === true)}
            />
            <span className="grid gap-0.5">
              <span className="font-medium">{t('editor.ownerEnabled')}</span>
              <span className="text-xs text-muted-foreground">{t('editor.ownerEnabledHint')}</span>
            </span>
          </label>
          {editing && (
            <label className="flex items-start gap-2 text-sm">
              <Checkbox
                className="mt-0.5"
                checked={isActive}
                onCheckedChange={(checked) => setIsActive(checked === true)}
              />
              <span className="grid gap-0.5">
                <span className="font-medium">{t('editor.isActive')}</span>
                <span className="text-xs text-muted-foreground">{t('editor.isActiveHint')}</span>
              </span>
            </label>
          )}
        </div>
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

interface TitleFieldSelectProps {
  id: string
  recordType: string
  currentKey: string
  value: string
  onChange: (value: string) => void
}

function TitleFieldSelect({ id, recordType, currentKey, value, onChange }: TitleFieldSelectProps) {
  const { t } = useTranslation('objects')
  const schema = useFieldSchema(recordType)
  const options = useMemo(
    () =>
      (schema.data?.fields ?? []).filter(
        (field) =>
          !field.is_system &&
          (field.is_active || field.key === currentKey) &&
          TITLE_FIELD_TYPES.includes(field.field_type),
      ),
    [schema.data, currentKey],
  )

  return (
    <div className="grid gap-1.5">
      <Label htmlFor={id}>{t('editor.titleField')}</Label>
      <NativeSelect
        id={id}
        value={value}
        disabled={schema.isPending}
        aria-describedby={`${id}-hint`}
        onChange={(event) => onChange(event.target.value)}
      >
        {options.length === 0 && <option value={value}>{value}</option>}
        {options.map((field) => (
          <option key={field.key} value={field.key}>
            {field.label}
          </option>
        ))}
      </NativeSelect>
      <p id={`${id}-hint`} className="text-xs text-muted-foreground">
        {t('editor.titleFieldHint')}
      </p>
    </div>
  )
}
