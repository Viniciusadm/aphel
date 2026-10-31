import { Plus, Trash2 } from 'lucide-react'
import {
  Controller,
  useFieldArray,
  useWatch,
  type Control,
  type FieldErrors,
  type UseFormRegister,
} from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import type { FieldFormContext, FieldFormValues } from '../lib/form'
import { optionValuesFromLabels } from '../lib/identifiers'
import { ColorInput, FieldMessage } from './FormControls'
import { SortableList } from './SortableList'
import { describedBy } from '../lib/aria'

interface DraftOptionsEditorProps {
  control: Control<FieldFormValues, FieldFormContext, FieldFormValues>
  register: UseFormRegister<FieldFormValues>
  errors: FieldErrors<FieldFormValues>
}

export function DraftOptionsEditor({ control, register, errors }: DraftOptionsEditorProps) {
  const { t } = useTranslation('fields')
  const { fields, append, remove, move } = useFieldArray({ control, name: 'options' })
  const options = useWatch({ control, name: 'options' })
  const values = optionValuesFromLabels(options.map((option) => option.label))

  return (
    <fieldset className="grid gap-3 rounded-lg border p-4">
      <legend className="px-1 text-sm font-medium">{t('options.title')}</legend>
      <p className="text-xs text-muted-foreground">{t('options.description')}</p>
      {fields.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t('options.empty')}</p>
      ) : (
        <SortableList
          items={fields}
          getId={(item) => item.id}
          getLabel={(item) => {
            const index = fields.findIndex((candidate) => candidate.id === item.id)
            return options[index]?.label || t('options.label', { position: index + 1 })
          }}
          onReorder={(_ids, from, to) => move(from, to)}
          label={t('options.title')}
          renderItem={(item, handle, index) => {
            const labelId = `field-option-${index}-label`
            const colorId = `field-option-${index}-color`
            const labelError = errors.options?.[index]?.label?.message
            const colorError = errors.options?.[index]?.color?.message
            const label = options[index]?.label.trim()
            return (
              <div className="flex flex-wrap items-start gap-2 rounded-md border bg-background p-2" data-option={item.id}>
                {handle}
                <div className="grid min-w-40 flex-1 gap-1">
                  <Input placeholder={t('options.labelPlaceholder')}
                    id={labelId}
                    aria-label={t('options.label', { position: index + 1 })}
                    aria-invalid={labelError ? true : undefined}
                    aria-describedby={describedBy(labelId, labelError, label ? 'value' : undefined)}
                    {...register(`options.${index}.label`)}
                  />
                  {labelError ? (
                    <FieldMessage id={`${labelId}-error`} message={labelError} />
                  ) : (
                    label && (
                      <p id={`${labelId}-hint`} className="text-xs text-muted-foreground">
                        {t('options.valueHint', { value: values[index] })}
                      </p>
                    )
                  )}
                </div>
                <div className="grid gap-1">
                  <Controller
                    control={control}
                    name={`options.${index}.color`}
                    render={({ field }) => (
                      <ColorInput
                        id={colorId}
                        value={field.value}
                        label={t('options.color', { position: index + 1 })}
                        pickerLabel={t('options.colorPicker', { position: index + 1 })}
                        invalid={Boolean(colorError)}
                        describedBy={describedBy(colorId, colorError)}
                        onChange={field.onChange}
                      />
                    )}
                  />
                  <FieldMessage id={`${colorId}-error`} message={colorError} />
                </div>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  aria-label={t('options.remove', { position: index + 1 })}
                  onClick={() => remove(index)}
                >
                  <Trash2 />
                </Button>
              </div>
            )
          }}
        />
      )}
      <div>
        <Button type="button" variant="outline" size="sm" onClick={() => append({ label: '', color: '' })}>
          <Plus />
          {t('options.add')}
        </Button>
      </div>
    </fieldset>
  )
}
