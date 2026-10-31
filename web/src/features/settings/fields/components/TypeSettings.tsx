import { Controller, type Control, type FieldErrors, type UseFormRegister } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import {
  RECORD_TYPES,
  hasOptions,
  isNumeric,
  supportsDefaultValue,
  supportsMaxLength,
  textLengthLimit,
  type FieldType,
} from '../lib/field-types'
import type { FieldFormContext, FieldFormValues } from '../lib/form'
import { ChoiceSelect, FieldMessage, FormField, type Choice } from './FormControls'
import { describedBy } from '../lib/aria'
import { Checkbox } from '@/components/ui/checkbox'

interface TypeSettingsProps {
  fieldType: FieldType
  control: Control<FieldFormValues, FieldFormContext, FieldFormValues>
  register: UseFormRegister<FieldFormValues>
  errors: FieldErrors<FieldFormValues>
  optionChoices: Choice[]
}

const DEFAULT_INPUT_TYPES: Partial<Record<FieldType, string>> = {
  email: 'email',
  url: 'url',
  phone: 'tel',
  date: 'date',
  datetime: 'datetime-local',
}

export function TypeSettings({ fieldType, control, register, errors, optionChoices }: TypeSettingsProps) {
  const { t } = useTranslation('fields')
  const configErrors = errors.config
  const hasSettings =
    isNumeric(fieldType) ||
    supportsMaxLength(fieldType) ||
    fieldType === 'phone' ||
    fieldType === 'record' ||
    supportsDefaultValue(fieldType)
  if (!hasSettings) return null

  const textInput = (
    name: 'min' | 'max' | 'decimals' | 'currency' | 'maxLength' | 'pattern' | 'defaultCountry',
    label: string,
    hint?: string,
    inputMode?: 'decimal' | 'numeric',
  ) => {
    const id = `field-config-${name}`
    const error = configErrors?.[name]?.message
    return (
      <FormField id={id} label={label} hint={hint} error={error}>
        <Input placeholder={t('config.valuePlaceholder')}
          id={id}
          inputMode={inputMode}
          aria-invalid={error ? true : undefined}
          aria-describedby={describedBy(id, error, hint)}
          {...register(`config.${name}`)}
        />
      </FormField>
    )
  }

  return (
    <fieldset className="grid gap-4 rounded-lg border p-4">
      <legend className="px-1 text-sm font-medium">{t('editor.settings')}</legend>
      <div className="grid gap-4 sm:grid-cols-2">
        {isNumeric(fieldType) && (
          <>
            {textInput('min', t('config.min'), undefined, 'decimal')}
            {textInput('max', t('config.max'), undefined, 'decimal')}
            {textInput('decimals', t('config.decimals'), t('config.decimalsHint'), 'numeric')}
          </>
        )}
        {fieldType === 'currency' && textInput('currency', t('config.currency'), t('config.currencyHint'))}
        {supportsMaxLength(fieldType) &&
          textInput(
            'maxLength',
            t('config.maxLength'),
            `${t('config.maxLengthHint')} (1–${textLengthLimit(fieldType)})`,
            'numeric',
          )}
        {fieldType === 'text' && textInput('pattern', t('config.pattern'), t('config.patternHint'))}
        {fieldType === 'phone' && textInput('defaultCountry', t('config.defaultCountry'), t('config.defaultCountryHint'))}
        {fieldType === 'record' && (
          <Controller
            control={control}
            name="config.targetRecordType"
            render={({ field, fieldState }) => (
              <FormField id="field-config-target" label={t('config.targetRecordType')} error={fieldState.error?.message}>
                <ChoiceSelect
                  id="field-config-target"
                  value={field.value}
                  placeholder={t('config.targetRecordTypePlaceholder')}
                  choices={RECORD_TYPES.map((recordType) => ({
                    value: recordType,
                    label: t(`recordTypes.${recordType}`),
                  }))}
                  invalid={Boolean(fieldState.error)}
                  describedBy={describedBy('field-config-target', fieldState.error?.message)}
                  onChange={field.onChange}
                />
              </FormField>
            )}
          />
        )}
        {supportsDefaultValue(fieldType) && (
          <DefaultValueInput
            fieldType={fieldType}
            control={control}
            register={register}
            errors={errors}
            optionChoices={optionChoices}
          />
        )}
      </div>
    </fieldset>
  )
}

function DefaultValueInput({ fieldType, control, register, errors, optionChoices }: TypeSettingsProps) {
  const { t } = useTranslation('fields')
  const id = 'field-config-default'
  const hint = t('config.defaultValueHint')
  const error = errors.config?.defaultValue?.message

  if (fieldType === 'multi_select') {
    const optionsError = errors.config?.defaultOptions?.message
    return (
      <Controller
        control={control}
        name="config.defaultOptions"
        render={({ field }) => (
          <fieldset
            className="grid content-start gap-2 sm:col-span-2"
            aria-describedby={describedBy(id, optionsError, hint)}
          >
            <legend className="mb-2 text-sm font-medium">{t('config.defaultValue')}</legend>
            {optionChoices.length === 0 ? (
              <p className="text-xs text-muted-foreground">{t('config.noOptions')}</p>
            ) : (
              <div className="flex flex-wrap gap-x-4 gap-y-2">
                {optionChoices.map((choice) => (
                  <label key={choice.value} className="flex items-center gap-2 text-sm">
                    <Checkbox
                      checked={field.value.includes(choice.value)}
                      onCheckedChange={(checked) =>
                        field.onChange(
                          checked === true
                            ? [...field.value, choice.value]
                            : field.value.filter((value) => value !== choice.value),
                        )
                      }
                    />
                    {choice.label}
                  </label>
                ))}
              </div>
            )}
            {optionsError ? (
              <FieldMessage id={`${id}-error`} message={optionsError} />
            ) : (
              <p id={`${id}-hint`} className="text-xs text-muted-foreground">
                {hint}
              </p>
            )}
          </fieldset>
        )}
      />
    )
  }

  if (fieldType === 'checkbox' || hasOptions(fieldType)) {
    const choices: Choice[] =
      fieldType === 'checkbox'
        ? [
            { value: 'true', label: t('config.checked') },
            { value: 'false', label: t('config.unchecked') },
          ]
        : optionChoices
    return (
      <Controller
        control={control}
        name="config.defaultValue"
        render={({ field }) => (
          <FormField id={id} label={t('config.defaultValue')} hint={hint} error={error}>
            <ChoiceSelect
              id={id}
              value={field.value}
              noneLabel={t('config.noDefault')}
              choices={choices.filter((choice) => choice.value !== '')}
              invalid={Boolean(error)}
              describedBy={describedBy(id, error, hint)}
              onChange={field.onChange}
            />
          </FormField>
        )}
      />
    )
  }

  return (
    <FormField id={id} label={t('config.defaultValue')} hint={hint} error={error}>
      <Input placeholder={t('config.defaultValuePlaceholder')}
        id={id}
        type={DEFAULT_INPUT_TYPES[fieldType] ?? 'text'}
        inputMode={isNumeric(fieldType) ? 'decimal' : undefined}
        aria-invalid={error ? true : undefined}
        aria-describedby={describedBy(id, error, hint)}
        {...register('config.defaultValue')}
      />
    </FormField>
  )
}
