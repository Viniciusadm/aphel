import { ArrowDown, ArrowUp, Trash2 } from 'lucide-react'
import { useFormContext, useWatch } from 'react-hook-form'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { LEAD_FORMS_KEY, FIELD_KINDS, FULL_NAME_TARGET, type LeadFormOptions } from './api'
import { slugify, type LeadFormValues } from './form'

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
  const prefix = `lead-form-field-${index}`
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
        <div className="grid gap-1.5">
          <Label htmlFor={`${prefix}-label`}>{t('editor.label')}</Label>
          <Input placeholder={t('editor.labelPlaceholder')} id={`${prefix}-label`} {...register(`fields.${index}.label`, { onBlur: fillKey })} />
          {errors?.label && <p className="text-xs text-destructive">{message(errors.label.message)}</p>}
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${prefix}-key`}>{t('editor.key')}</Label>
          <Input placeholder={t('editor.keyPlaceholder')} id={`${prefix}-key`} className="font-mono" {...register(`fields.${index}.key`)} />
          {errors?.key && <p className="text-xs text-destructive">{message(errors.key.message)}</p>}
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${prefix}-kind`}>{t('editor.kind')}</Label>
          <NativeSelect id={`${prefix}-kind`} {...register(`fields.${index}.kind`)}>
            {FIELD_KINDS.map((option) => (
              <option key={option} value={option}>
                {t(`kinds.${option}`)}
              </option>
            ))}
          </NativeSelect>
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${prefix}-target`}>{t('editor.target')}</Label>
          <NativeSelect id={`${prefix}-target`} {...register(`fields.${index}.target`)}>
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
          </NativeSelect>
        </div>
        {kind !== 'checkbox' && (
          <div className="grid gap-1.5 sm:col-span-2">
            <Label htmlFor={`${prefix}-placeholder`}>
              {kind === 'select' ? t('editor.selectPrompt') : t('editor.placeholder')}
            </Label>
            <Input id={`${prefix}-placeholder`} {...register(`fields.${index}.placeholder`)} />
            {errors?.placeholder && <p className="text-xs text-destructive">{message(errors.placeholder.message)}</p>}
          </div>
        )}
        {kind === 'select' && (
          <div className="grid gap-1.5 sm:col-span-2">
            <Label htmlFor={`${prefix}-options`}>{t('editor.options')}</Label>
            <Textarea placeholder={t('editor.optionsPlaceholder')} id={`${prefix}-options`} rows={3} {...register(`fields.${index}.options`)} />
            <p className="text-xs text-muted-foreground">{t('editor.optionsHelp')}</p>
            {errors?.options && <p className="text-xs text-destructive">{message(errors.options.message)}</p>}
          </div>
        )}
      </div>
      <label className="flex items-center gap-2 text-sm">
        <input type="checkbox" className="size-4 accent-primary" {...register(`fields.${index}.required`)} />
        {t('editor.required')}
      </label>
      {errors?.required && <p className="text-xs text-destructive">{message(errors.required.message)}</p>}
    </li>
  )
}
