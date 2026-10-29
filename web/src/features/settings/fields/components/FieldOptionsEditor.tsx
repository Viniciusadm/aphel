import { Check, Plus } from 'lucide-react'
import { useState, type KeyboardEvent, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { toast } from '@/components/ui/toast-store'
import { cn } from '@/lib/utils'
import { versionOf } from '../../api'
import { fieldViolations } from '../../lib/errors'
import {
  fieldsApi,
  sortByPosition,
  useFieldMutation,
  useFieldsCache,
  type Field,
  type FieldOption,
  type UpdateOptionRequest,
} from '../api'
import { LABEL_MAX_LENGTH } from '../lib/field-types'
import { HEX_COLOR_PATTERN } from '../lib/form'
import { errorMessageKey, violationMessageKey, type FieldsMessageKey } from '../lib/errors'
import { ColorInput, FieldMessage, FormAlert } from './FormControls'
import { SortableList } from './SortableList'
import { describedBy } from '../lib/aria'

interface OptionErrors {
  label?: FieldsMessageKey
  color?: FieldsMessageKey
  form?: FieldsMessageKey
}

function validateOption(label: string, color: string): OptionErrors {
  const errors: OptionErrors = {}
  const trimmed = label.trim()
  if (trimmed === '' || trimmed.length > LABEL_MAX_LENGTH) errors.label = 'validation.optionLabel'
  if (color.trim() !== '' && !HEX_COLOR_PATTERN.test(color.trim())) errors.color = 'validation.color'
  return errors
}

function hasErrors(errors: OptionErrors): boolean {
  return Boolean(errors.label || errors.color || errors.form)
}

function serverOptionErrors(error: unknown): OptionErrors {
  const errors: OptionErrors = {}
  if (isApiError(error)) {
    for (const violation of fieldViolations(error.details)) {
      if (violation.field === 'color') errors.color = violationMessageKey('color')
      else if (violation.field === 'label' || violation.field === 'value') {
        errors.label = violationMessageKey(violation.field)
      }
    }
    if (error.code === 'option_value_taken') errors.label = errorMessageKey(error)
  }
  if (!hasErrors(errors)) errors.form = errorMessageKey(error)
  return errors
}

function onEnter(action: () => void) {
  return (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key !== 'Enter') return
    event.preventDefault()
    action()
  }
}

interface FieldOptionsEditorProps {
  recordType: string
  field: Field
}

export function FieldOptionsEditor({ recordType, field }: FieldOptionsEditorProps) {
  const { t } = useTranslation('fields')
  const cache = useFieldsCache(recordType)
  const options = sortByPosition(field.options)
  const reorder = useFieldMutation(recordType, (ids: string[]) => fieldsApi.reorderOptions(recordType, field.id, ids))

  const handleReorder = (ids: string[]) => {
    const previous = field
    const byId = new Map(field.options.map((option) => [option.id, option]))
    const reordered = ids.flatMap((id, position) => {
      const option = byId.get(id)
      return option ? [{ ...option, position }] : []
    })
    cache.replace({ ...field, options: reordered })
    reorder.mutate(ids, {
      onError: (error) => {
        cache.replace(previous)
        toast({ title: t(errorMessageKey(error)), variant: 'destructive' })
      },
    })
  }

  return (
    <fieldset className="grid gap-3 rounded-lg border p-4">
      <legend className="px-1 text-sm font-medium">{t('options.title')}</legend>
      <p className="text-xs text-muted-foreground">
        {t('options.description')} {t('options.inactiveHint')}
      </p>
      {options.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t('options.empty')}</p>
      ) : (
        <SortableList
          items={options}
          getId={(option) => option.id}
          getLabel={(option) => option.label}
          onReorder={(ids) => handleReorder(ids)}
          label={t('options.title')}
          renderItem={(option, handle, index) => (
            <OptionRow
              key={`${option.id}-${option.label}-${option.color ?? ''}`}
              recordType={recordType}
              fieldId={field.id}
              fieldVersion={versionOf(field)}
              option={option}
              position={index + 1}
              handle={handle}
            />
          )}
        />
      )}
      <NewOptionForm recordType={recordType} fieldId={field.id} />
    </fieldset>
  )
}

interface OptionRowProps {
  recordType: string
  fieldId: string
  fieldVersion: string | null
  option: FieldOption
  position: number
  handle: ReactNode
}

function OptionRow({ recordType, fieldId, fieldVersion, option, position, handle }: OptionRowProps) {
  const { t } = useTranslation('fields')
  const [label, setLabel] = useState(option.label)
  const [color, setColor] = useState(option.color ?? '')
  const [errors, setErrors] = useState<OptionErrors>({})
  const update = useFieldMutation(recordType, (request: UpdateOptionRequest) =>
    fieldsApi.updateOption(recordType, fieldId, option.id, request, fieldVersion),
  )
  const labelChanged = label.trim() !== option.label
  const colorChanged = color.trim().toLowerCase() !== (option.color ?? '').toLowerCase()
  const dirty = labelChanged || colorChanged
  const labelId = `option-${option.id}-label`
  const colorId = `option-${option.id}-color`

  const save = async () => {
    if (!dirty || update.isPending) return
    const validation = validateOption(label, color)
    setErrors(validation)
    if (hasErrors(validation)) return
    const request: UpdateOptionRequest = {
      ...(labelChanged ? { label: label.trim() } : {}),
      ...(colorChanged ? { color: color.trim() === '' ? null : color.trim().toLowerCase() } : {}),
    }
    try {
      await update.mutateAsync(request)
      toast({ title: t('toast.optionUpdated'), variant: 'success' })
    } catch (error) {
      setErrors(serverOptionErrors(error))
    }
  }

  const toggleActive = async (isActive: boolean) => {
    try {
      await update.mutateAsync({ is_active: isActive })
      toast({ title: t('toast.optionUpdated'), variant: 'success' })
    } catch (error) {
      toast({ title: t(errorMessageKey(error)), variant: 'destructive' })
    }
  }

  return (
    <div
      className={cn(
        'grid gap-2 rounded-md border bg-background p-2',
        !option.is_active && 'bg-muted/40 text-muted-foreground',
      )}
    >
      <div className="flex flex-wrap items-start gap-2">
        {handle}
        <div className="grid min-w-40 flex-1 gap-1">
          <Input
            id={labelId}
            aria-label={t('options.label', { position })}
            value={label}
            aria-invalid={errors.label ? true : undefined}
            aria-describedby={describedBy(labelId, errors.label, 'value')}
            onChange={(event) => setLabel(event.target.value)}
            onKeyDown={onEnter(() => void save())}
          />
          {errors.label ? (
            <FieldMessage id={`${labelId}-error`} message={errors.label} />
          ) : (
            <p id={`${labelId}-hint`} className="text-xs text-muted-foreground">
              {t('options.valueHint', { value: option.value })}
            </p>
          )}
        </div>
        <div className="grid gap-1">
          <ColorInput
            id={colorId}
            value={color}
            label={t('options.color', { position })}
            pickerLabel={t('options.colorPicker', { position })}
            invalid={Boolean(errors.color)}
            describedBy={describedBy(colorId, errors.color)}
            onChange={setColor}
            onKeyDown={onEnter(() => void save())}
          />
          <FieldMessage id={`${colorId}-error`} message={errors.color} />
        </div>
        <label className="flex h-9 items-center gap-2 text-sm">
          <input
            type="checkbox"
            className="size-4 accent-primary"
            checked={option.is_active}
            disabled={update.isPending}
            aria-label={t('options.activeLabel', { label: option.label })}
            onChange={(event) => void toggleActive(event.target.checked)}
          />
          <span aria-hidden>{t('options.active')}</span>
        </label>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          aria-label={t('options.save', { label: option.label })}
          disabled={!dirty || update.isPending}
          onClick={() => void save()}
        >
          <Check />
        </Button>
      </div>
      <FormAlert message={errors.form} />
    </div>
  )
}

interface NewOptionFormProps {
  recordType: string
  fieldId: string
}

function NewOptionForm({ recordType, fieldId }: NewOptionFormProps) {
  const { t } = useTranslation('fields')
  const [label, setLabel] = useState('')
  const [color, setColor] = useState('')
  const [errors, setErrors] = useState<OptionErrors>({})
  const add = useFieldMutation(recordType, () =>
    fieldsApi.addOption(recordType, fieldId, {
      label: label.trim(),
      ...(color.trim() ? { color: color.trim().toLowerCase() } : {}),
    }),
  )
  const labelId = 'new-option-label'
  const colorId = 'new-option-color'

  const submit = async () => {
    if (add.isPending) return
    const validation = validateOption(label, color)
    setErrors(validation)
    if (hasErrors(validation)) return
    try {
      await add.mutateAsync(undefined)
      setLabel('')
      setColor('')
      toast({ title: t('toast.optionAdded'), variant: 'success' })
    } catch (error) {
      setErrors(serverOptionErrors(error))
    }
  }

  return (
    <div className="grid gap-2 border-t pt-3">
      <div className="flex flex-wrap items-start gap-2">
        <div className="grid min-w-40 flex-1 gap-1">
          <Input
            id={labelId}
            aria-label={t('options.newLabel')}
            placeholder={t('options.newLabel')}
            value={label}
            aria-invalid={errors.label ? true : undefined}
            aria-describedby={describedBy(labelId, errors.label)}
            onChange={(event) => setLabel(event.target.value)}
            onKeyDown={onEnter(() => void submit())}
          />
          <FieldMessage id={`${labelId}-error`} message={errors.label} />
        </div>
        <div className="grid gap-1">
          <ColorInput
            id={colorId}
            value={color}
            label={t('options.newColor')}
            pickerLabel={t('options.newColorPicker')}
            invalid={Boolean(errors.color)}
            describedBy={describedBy(colorId, errors.color)}
            onChange={setColor}
            onKeyDown={onEnter(() => void submit())}
          />
          <FieldMessage id={`${colorId}-error`} message={errors.color} />
        </div>
        <Button type="button" variant="outline" onClick={() => void submit()} disabled={add.isPending}>
          <Plus />
          {t('options.add')}
        </Button>
      </div>
      <FormAlert message={errors.form} />
    </div>
  )
}
