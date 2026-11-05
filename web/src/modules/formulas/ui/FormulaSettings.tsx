import { useMemo, useRef, useState } from 'react'
import { Controller, useWatch, type Control, type FieldErrors, type UseFormRegister, type UseFormSetValue } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { Input } from '@/components/ui/input'
import { ChoiceSelect, describedBy, type FieldFormContext, type FieldFormValues, FormField } from '@/sdk'
import { useFormulaCatalog, useFormulaPreview, type FormulaPreviewRequest } from '@/sdk'
import { ExpressionEditor } from '@/sdk'
import { FormulaCatalogPanel, type FormulaSnippet } from '@/sdk'
import { FormulaPreviewPanel } from '@/modules/formulas/ui/FormulaPreviewPanel'
import { FormulaStatus } from '@/sdk'
import {
  FORMULA_PREVIEW_DEBOUNCE_MS,
  FORMULA_RESULT_TYPES,
  formulaUsesCurrency,
  formulaUsesDecimals,
  parseDecimals,
} from '@/sdk'
import { useDebounced } from '@/sdk'

interface FormulaSettingsProps {
  recordType: string
  fieldKey?: string
  control: Control<FieldFormValues, FieldFormContext, FieldFormValues>
  register: UseFormRegister<FieldFormValues>
  setValue: UseFormSetValue<FieldFormValues>
  errors: FieldErrors<FieldFormValues>
  serverMessages?: string[]
}

export function FormulaSettings({
  recordType,
  fieldKey,
  control,
  register,
  setValue,
  errors,
  serverMessages = [],
}: FormulaSettingsProps) {
  const { t } = useTranslation('formulas')
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)
  const [recordId, setRecordId] = useState<string | null>(null)
  const expression = useWatch({ control, name: 'config.expression' })
  const resultType = useWatch({ control, name: 'config.resultType' })
  const decimalsText = useWatch({ control, name: 'config.decimals' })
  const currency = useWatch({ control, name: 'config.currency' })
  const catalog = useFormulaCatalog(recordType)

  const decimals = formulaUsesDecimals(resultType) ? parseDecimals(decimalsText) : undefined
  const trimmed = expression.trim()
  const request: FormulaPreviewRequest | null = trimmed
    ? {
        record_type: recordType,
        expression,
        result_type: resultType,
        ...(decimals !== undefined ? { decimals } : {}),
        ...(recordId ? { record_id: recordId } : {}),
        ...(fieldKey ? { field_key: fieldKey } : {}),
      }
    : null
  const requestKey = request ? JSON.stringify(request) : ''
  const debouncedKey = useDebounced(requestKey, FORMULA_PREVIEW_DEBOUNCE_MS)
  const pending = requestKey !== debouncedKey
  const debouncedRequest = useMemo(
    () => (debouncedKey ? (JSON.parse(debouncedKey) as FormulaPreviewRequest) : null),
    [debouncedKey],
  )
  const preview = useFormulaPreview(debouncedRequest)
  const previewData = request ? preview.data : undefined
  const failed = preview.isError && !(isApiError(preview.error) && preview.error.code === 'record_not_found')

  const expressionError = errors.config?.expression?.message
  const resultTypeError = errors.config?.resultType?.message
  const expressionId = 'field-config-expression'
  const statusId = `${expressionId}-status`

  const insert = ({ text, caret }: FormulaSnippet) => {
    const element = textareaRef.current
    const start = element?.selectionStart ?? expression.length
    const end = element?.selectionEnd ?? expression.length
    const next = `${expression.slice(0, start)}${text}${expression.slice(end)}`
    setValue('config.expression', next, { shouldDirty: true, shouldValidate: Boolean(expressionError) })
    const position = start + (caret ?? text.length)
    requestAnimationFrame(() => {
      element?.focus()
      element?.setSelectionRange(position, position)
    })
  }

  return (
    <fieldset className="grid min-w-0 gap-4 rounded-lg border p-4">
      <legend className="px-1 text-sm font-medium">{t('settings.title')}</legend>
      <p className="text-sm text-muted-foreground">{t('settings.description')}</p>
      <div className="grid min-w-0 gap-4 sm:grid-cols-3">
        <Controller
          control={control}
          name="config.resultType"
          render={({ field }) => (
            <FormField
              id="field-config-result-type"
              label={t('settings.resultType')}
              hint={t('settings.resultTypeHint')}
              error={resultTypeError}
            >
              <ChoiceSelect
                id="field-config-result-type"
                value={field.value}
                choices={FORMULA_RESULT_TYPES.map((type) => ({ value: type, label: t(`resultTypes.${type}`) }))}
                invalid={Boolean(resultTypeError)}
                describedBy={describedBy('field-config-result-type', resultTypeError, t('settings.resultTypeHint'))}
                onChange={field.onChange}
              />
            </FormField>
          )}
        />
        {formulaUsesDecimals(resultType) && (
          <FormField
            id="field-config-decimals"
            label={t('settings.decimals')}
            hint={t('settings.decimalsHint')}
            error={errors.config?.decimals?.message}
          >
            <Input placeholder={t('settings.decimalsPlaceholder')}
              id="field-config-decimals"
              inputMode="numeric"
              aria-invalid={errors.config?.decimals ? true : undefined}
              aria-describedby={describedBy(
                'field-config-decimals',
                errors.config?.decimals?.message,
                t('settings.decimalsHint'),
              )}
              {...register('config.decimals')}
            />
          </FormField>
        )}
        {formulaUsesCurrency(resultType) && (
          <FormField
            id="field-config-currency"
            label={t('settings.currency')}
            hint={t('settings.currencyHint')}
            error={errors.config?.currency?.message}
          >
            <Input
              id="field-config-currency"
              placeholder="BRL"
              maxLength={3}
              className="uppercase"
              aria-invalid={errors.config?.currency ? true : undefined}
              aria-describedby={describedBy(
                'field-config-currency',
                errors.config?.currency?.message,
                t('settings.currencyHint'),
              )}
              {...register('config.currency')}
            />
          </FormField>
        )}
      </div>
      <FormField id={expressionId} label={t('editor.label')} hint={t('editor.hint')} error={expressionError}>
        <Controller
          control={control}
          name="config.expression"
          render={({ field }) => (
            <ExpressionEditor
              id={expressionId}
              name={field.name}
              value={field.value}
              errors={previewData && !previewData.valid ? previewData.errors : []}
              invalid={Boolean(expressionError) || (previewData ? !previewData.valid : false)}
              describedBy={[describedBy(expressionId, expressionError, t('editor.hint')), statusId]
                .filter(Boolean)
                .join(' ')}
              textareaRef={(element) => {
                field.ref(element)
                textareaRef.current = element
              }}
              onChange={field.onChange}
              onBlur={field.onBlur}
            />
          )}
        />
      </FormField>
      {serverMessages.length > 0 && (
        <ul role="alert" className="grid gap-1 text-sm text-destructive">
          {serverMessages.map((message) => (
            <li key={message} className="break-words">
              {t('editor.serverError', { message })}
            </li>
          ))}
        </ul>
      )}
      <FormulaStatus
        id={statusId}
        preview={previewData}
        expression={expression}
        validating={Boolean(request) && (pending || preview.isFetching)}
        failed={Boolean(request) && failed && !pending}
        catalog={catalog.data}
      />
      <FormulaCatalogPanel recordType={recordType} onInsert={insert} />
      <FormulaPreviewPanel
        recordType={recordType}
        recordId={recordId}
        onRecordChange={setRecordId}
        preview={previewData}
        error={request ? preview.error : null}
        loading={Boolean(request) && (pending || preview.isFetching)}
        format={{ resultType, decimals, currency }}
      />
    </fieldset>
  )
}
