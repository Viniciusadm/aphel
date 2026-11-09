import { Controller, useWatch, type Control, type FieldErrors, type UseFormRegister } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { MaskedInput } from '@/components/ui/masked-input'
import { PhoneFieldInput } from '@/components/fields/LazyPhoneFieldInput'
import { useRecordFormatting } from '@/features/records/formatting'
import { TEXT_MASK_KINDS, isTextMaskKind } from '@/lib/text-mask'
import { useRecordTypeOptions } from '../lib/record-types'
import {
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
  date: 'date',
  datetime: 'datetime-local',
}

export function TypeSettings({ fieldType, control, register, errors, optionChoices }: TypeSettingsProps) {
  const { t } = useTranslation('fields')
  const recordTypeOptions = useRecordTypeOptions()
  const configErrors = errors.config
  const hasSettings =
    isNumeric(fieldType) ||
    supportsMaxLength(fieldType) ||
    fieldType === 'phone' ||
    fieldType === 'record' ||
    supportsDefaultValue(fieldType)
  if (!hasSettings) return null

  const textInput = (
    name: 'min' | 'max' | 'decimals' | 'currency' | 'maxLength' | 'pattern',
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
        {fieldType === 'text' && (
          <Controller
            control={control}
            name="config.mask"
            render={({ field, fieldState }) => (
              <FormField id="field-config-mask" label={t('config.mask')} hint={t('config.maskHint')} error={fieldState.error?.message}>
                <ChoiceSelect
                  id="field-config-mask"
                  value={field.value}
                  noneLabel={t('config.maskNone')}
                  choices={TEXT_MASK_KINDS.map((mask) => ({ value: mask, label: t(`config.masks.${mask}`) }))}
                  invalid={Boolean(fieldState.error)}
                  describedBy={describedBy('field-config-mask', fieldState.error?.message, t('config.maskHint'))}
                  onChange={field.onChange}
                />
              </FormField>
            )}
          />
        )}
        {fieldType === 'phone' && (
          <Controller
            control={control}
            name="config.defaultCountry"
            render={({ field, fieldState }) => (
              <FormField
                id="field-config-defaultCountry"
                label={t('config.defaultCountry')}
                hint={t('config.defaultCountryHint')}
                error={fieldState.error?.message}
              >
                <Input
                  id="field-config-defaultCountry"
                  placeholder="BR"
                  maxLength={2}
                  autoComplete="off"
                  aria-invalid={fieldState.error ? true : undefined}
                  aria-describedby={describedBy('field-config-defaultCountry', fieldState.error?.message, t('config.defaultCountryHint'))}
                  value={field.value}
                  onBlur={field.onBlur}
                  onChange={(event) => field.onChange(event.target.value.replace(/[^A-Za-z]/g, '').toUpperCase())}
                />
              </FormField>
            )}
          />
        )}
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
                  choices={recordTypeOptions}
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
  const formatting = useRecordFormatting()
  const mask = useWatch({ control, name: 'config.mask' })
  const defaultCountry = useWatch({ control, name: 'config.defaultCountry' })
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

  if (fieldType === 'phone' || (fieldType === 'text' && isTextMaskKind(mask))) {
    return (
      <Controller
        control={control}
        name="config.defaultValue"
        render={({ field }) => (
          <FormField id={id} label={t('config.defaultValue')} hint={hint} error={error}>
            {fieldType === 'phone' ? (
              <PhoneFieldInput
                id={id}
                locale={formatting.locale}
                defaultCountry={defaultCountry || null}
                fieldLabel={t('config.defaultValue')}
                invalid={Boolean(error)}
                aria-describedby={describedBy(id, error, hint)}
                value={field.value || null}
                onBlur={field.onBlur}
                onChange={(next) => field.onChange(next ?? '')}
              />
            ) : (
              <MaskedInput
                id={id}
                mask={isTextMaskKind(mask) ? mask : 'digits'}
                aria-invalid={error ? true : undefined}
                aria-describedby={describedBy(id, error, hint)}
                value={field.value}
                onBlur={field.onBlur}
                onValueChange={field.onChange}
              />
            )}
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
