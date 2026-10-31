import { ArrowDown, ArrowUp, Trash2 } from 'lucide-react'
import { Controller, useFormContext, useWatch } from 'react-hook-form'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { LEAD_FORMS_KEY, FIELD_KINDS, FULL_NAME_TARGET, type LeadFormOptions } from './api'
import { slugify, type LeadFormValues } from './form'
import { Checkbox } from '@/components/ui/checkbox'
import { Field } from '@/components/ui/field'

interface FieldEditorProps {
  index: number
  total: number
  targets: LeadFormOptions['contact_fields']
  onMove: (from: number, to: number) => void
  onRemove: () => void
}

export function FieldEditor({ index, total, targets, onMove, onRemove }: FieldEditorProps) {
  const t = useModuleTranslation(LEAD_FORMS_KEY)
  const { register, control, getValues, setValue, formState } = useFormContext<LeadFormValues>()
  const kind = useWatch({ control, name: `fields.${index}.kind` })
  const errors = formState.errors.fields?.[index]
  const compatibleTargets = targets.filter((target) =>
    kind === 'checkbox' ? target.field_type === 'checkbox' : target.field_type !== 'checkbox',
  )
  const message = (key?: string) => (key ? t(key) : null)

  const fillKey = () => {
    const key = getValues(`fields.${index}.key`)
    if (key && !/^field_\d+$/.test(key)) return
    const slug = slugify(getValues(`fields.${index}.label`))
    if (slug) setValue(`fields.${index}.key`, slug, { shouldDirty: true })
  }

  return (
    <li className="grid gap-3 rounded-md border bg-card p-3">
      <div className="flex items-center justify-between gap-2">
        <span className="text-sm font-medium">{t('editor.fieldNumber', { number: index + 1 })}</span>
        <div className="flex gap-1">
          <Button
            type="button"
            size="icon"
            variant="ghost"
            disabled={index === 0}
            aria-label={t('editor.moveUp')}
            onClick={() => onMove(index, index - 1)}
          >
            <ArrowUp aria-hidden="true" />
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            disabled={index === total - 1}
            aria-label={t('editor.moveDown')}
            onClick={() => onMove(index, index + 1)}
          >
            <ArrowDown aria-hidden="true" />
          </Button>
          <Button type="button" size="icon" variant="ghost" aria-label={t('editor.removeField')} onClick={onRemove}>
            <Trash2 aria-hidden="true" />
          </Button>
        </div>
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <Field label={t('editor.label')}>
          {(control) => <>
            <Input placeholder={t('editor.labelPlaceholder')} {...control} {...register(`fields.${index}.label`, { onBlur: fillKey })} />
          {errors?.label && <p className="text-sm text-destructive">{message(errors.label.message)}</p>}
          </>}
        </Field>
        <Field label={t('editor.key')}>
          {(control) => <>
            <Input placeholder={t('editor.keyPlaceholder')} {...control} className="font-mono" {...register(`fields.${index}.key`)} />
          {errors?.key && <p className="text-sm text-destructive">{message(errors.key.message)}</p>}
          </>}
        </Field>
        <Field label={t('editor.kind')}>
          {(control) => <NativeSelect {...control} {...register(`fields.${index}.kind`)}>
            {FIELD_KINDS.map((option) => (
              <option key={option} value={option}>
                {t(`kinds.${option}`)}
              </option>
            ))}
          </NativeSelect>}
        </Field>
        <Field label={t('editor.target')}>
          {(control) => <NativeSelect {...control} {...register(`fields.${index}.target`)}>
            <option value="">{t('editor.noTarget')}</option>
            {compatibleTargets.map((target) => (
              <option key={target.key} value={target.key}>
                {target.key === FULL_NAME_TARGET
                  ? t('editor.fullNameTarget')
                  : target.custom
                    ? t('editor.customTarget', { label: target.label })
                    : target.label}
              </option>
            ))}
          </NativeSelect>}
        </Field>
        {kind !== 'checkbox' && (
          <Field label={kind === 'select' ? t('editor.selectPrompt') : t('editor.placeholder')} className="sm:col-span-2">
            {(control) => <>
              <Input {...control} {...register(`fields.${index}.placeholder`)} />
            {errors?.placeholder && <p className="text-sm text-destructive">{message(errors.placeholder.message)}</p>}
            </>}
          </Field>
        )}
        {kind === 'select' && (
          <Field label={t('editor.options')} className="sm:col-span-2">
            {(control) => <>
              <Textarea placeholder={t('editor.optionsPlaceholder')} {...control} rows={3} {...register(`fields.${index}.options`)} />
            <p className="text-xs text-muted-foreground">{t('editor.optionsHelp')}</p>
            {errors?.options && <p className="text-sm text-destructive">{message(errors.options.message)}</p>}
            </>}
          </Field>
        )}
      </div>
      <label className="flex items-center gap-2 text-sm">
        <Controller
          control={control}
          name={`fields.${index}.required`}
          render={({ field }) => (
            <Checkbox checked={field.value === true} onCheckedChange={(checked) => field.onChange(checked === true)} />
          )}
        />
        {t('editor.required')}
      </label>
      {errors?.required && <p className="text-sm text-destructive">{message(errors.required.message)}</p>}
    </li>
  )
}
