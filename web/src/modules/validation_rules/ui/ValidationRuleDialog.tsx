import { DialogForm } from '@/components/ui/dialog-form'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { Loader2 } from 'lucide-react'
import { useId, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { RecordFieldInput } from '@/components/fields'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { useFieldSchema } from '@/features/records/api'

import { activeFields, fieldViolations } from '@/features/records/fields'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useFormulaCatalog } from '@/modules/formulas/ui/api'
import { ExpressionEditor } from '@/modules/formulas/ui/ExpressionEditor'
import { FormulaCatalogPanel, type FormulaSnippet } from '@/modules/formulas/ui/FormulaCatalogPanel'
import { FormulaStatus } from '@/modules/formulas/ui/FormulaStatus'
import { FORMULA_EXPRESSION_MAX_LENGTH, FORMULA_PREVIEW_DEBOUNCE_MS } from '@/modules/formulas/ui/result-types'
import { useDebounced } from '@/modules/formulas/ui/use-debounced'
import { useValidationRulePreview, validationRulesApi } from '@/modules/validation_rules/ui/api'
import {
  VALIDATION_RULE_TIMINGS,
  type ValidationRule,
  type ValidationRulePreviewRequest,
  type ValidationRuleTiming,
} from '@/modules/validation_rules/ui/types'
import { Field } from '@/components/ui/field'

const NAME_MAX_LENGTH = 120
const MESSAGE_MAX_LENGTH = 500

interface ValidationRuleDialogProps {
  open: boolean
  recordType: string
  rule: ValidationRule | null
  onOpenChange: (open: boolean) => void
  onSaved: (created: boolean) => void
}

export function ValidationRuleDialog({ open, onOpenChange, ...props }: ValidationRuleDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="2xl">
        {open && <ValidationRuleForm key={props.rule?.id ?? 'new'} onOpenChange={onOpenChange} {...props} />}
      </DialogContent>
    </Dialog>
  )
}

type FormProps = Omit<ValidationRuleDialogProps, 'open'>

function ValidationRuleForm({ recordType, rule, onOpenChange, onSaved }: FormProps) {
  const { t } = useTranslation('validation-rules')
  const schema = useFieldSchema(recordType)
  const catalog = useFormulaCatalog(recordType)
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)
  const [name, setName] = useState(rule?.name ?? '')
  const [key, setKey] = useState(rule?.key ?? '')
  const [message, setMessage] = useState(rule?.message ?? '')
  const [field, setField] = useState(rule?.field ?? '')
  const [timing, setTiming] = useState<ValidationRuleTiming>(rule?.applies_on ?? 'always')
  const [active, setActive] = useState(rule?.is_active ?? true)
  const [expression, setExpression] = useState(rule?.expression ?? '')
  const [recordId, setRecordId] = useState<string | null>(null)
  const [submitted, setSubmitted] = useState(false)
  const [saving, setSaving] = useState(false)
  const [serverErrors, setServerErrors] = useState<Record<string, string>>({})
  const [generalError, setGeneralError] = useState<string | null>(null)
  const ids = {
    name: useId(),
    key: useId(),
    message: useId(),
    field: useId(),
    timing: useId(),
    active: useId(),
    expression: useId(),
    record: useId(),
  }
  const statusId = `${ids.expression}-status`

  const fields = schema.data ? activeFields(schema.data) : []
  const trimmed = expression.trim()
  const request: ValidationRulePreviewRequest | null = trimmed
    ? { record_type: recordType, expression, ...(recordId ? { record_id: recordId } : {}) }
    : null
  const requestKey = request ? JSON.stringify(request) : ''
  const debouncedKey = useDebounced(requestKey, FORMULA_PREVIEW_DEBOUNCE_MS)
  const pending = requestKey !== debouncedKey
  const debouncedRequest = useMemo(
    () => (debouncedKey ? (JSON.parse(debouncedKey) as ValidationRulePreviewRequest) : null),
    [debouncedKey],
  )
  const preview = useValidationRulePreview(debouncedRequest)
  const formula = request ? preview.data?.formula : undefined
  const recordMissing = isApiError(preview.error) && preview.error.code === 'record_not_found'
  const validating = Boolean(request) && (pending || preview.isFetching)

  const nameError = submitted && !name.trim() ? t('dialog.required') : serverErrors.name
  const messageError = submitted && !message.trim() ? t('dialog.required') : serverErrors.message
  const expressionError = submitted && !trimmed ? t('dialog.required') : serverErrors.expression

  const insert = ({ text, caret }: FormulaSnippet) => {
    const element = textareaRef.current
    const start = element?.selectionStart ?? expression.length
    const end = element?.selectionEnd ?? expression.length
    setExpression(`${expression.slice(0, start)}${text}${expression.slice(end)}`)
    const position = start + (caret ?? text.length)
    requestAnimationFrame(() => {
      element?.focus()
      element?.setSelectionRange(position, position)
    })
  }

  const submit = async () => {
    setSubmitted(true)
    if (!name.trim() || !message.trim() || !trimmed) return
    setSaving(true)
    setServerErrors({})
    setGeneralError(null)
    try {
      if (rule) {
        await validationRulesApi.update(
          rule.id,
          {
            name: name.trim(),
            message: message.trim(),
            expression,
            field: field || null,
            applies_on: timing,
            is_active: active,
          },
          rule.updated_at,
        )
      } else {
        await validationRulesApi.create({
          record_type: recordType,
          ...(key.trim() ? { key: key.trim() } : {}),
          name: name.trim(),
          message: message.trim(),
          expression,
          field: field || null,
          applies_on: timing,
          is_active: active,
        })
      }
      onSaved(!rule)
    } catch (error) {
      if (isApiError(error) && error.status === 409) {
        setGeneralError(t('toasts.conflict'))
      } else {
        const violations = fieldViolations(isApiError(error) ? error.details : null)
        const mapped: Record<string, string> = {}
        for (const violation of violations) mapped[violation.field] ??= violation.message
        setServerErrors(mapped)
        if (violations.length === 0) setGeneralError(t('toasts.error'))
      }
    } finally {
      setSaving(false)
    }
  }

  let testResult
  if (!recordId) testResult = <span className="text-muted-foreground">{t('test.pick')}</span>
  else if (recordMissing) testResult = <span className="text-destructive">{t('test.recordNotFound')}</span>
  else if (validating && !preview.data)
    testResult = <Loader2 className="size-4 animate-spin text-muted-foreground" aria-hidden="true" />
  else if (preview.data?.fails === true) testResult = <span className="font-medium text-destructive">{t('test.fails')}</span>
  else if (preview.data?.fails === false) testResult = <span className="font-medium text-success">{t('test.passes')}</span>
  else testResult = <span className="text-muted-foreground">{t('test.notEvaluated')}</span>

  return (
    <DialogForm
      className="min-w-0"
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
      title={rule ? t('dialog.editTitle') : t('dialog.newTitle')}
      description={t('dialog.description')}
      cancelLabel={t('dialog.cancel')}
      submitLabel={t('dialog.save')}
      onCancel={() => onOpenChange(false)}
      pending={saving}
      submitDisabled={saving}
    >
      <div className="grid min-w-0 gap-4 sm:grid-cols-2">
        <Field label={t('dialog.name')} className="min-w-0">
          {(control) => <>
            <Input placeholder={t('dialog.namePlaceholder')}
            {...control}
            value={name}
            maxLength={NAME_MAX_LENGTH}
            aria-invalid={nameError ? true : undefined}
            onChange={(event) => setName(event.target.value)}
          />
          {nameError && <SharedFieldError>{nameError}</SharedFieldError>}
          </>}
        </Field>
        <Field label={t('dialog.key')} className="min-w-0">
          {(control) => <>
            <Input placeholder={t('dialog.keyPlaceholder')}
            {...control}
            value={key}
            disabled={Boolean(rule)}
            maxLength={64}
            className="font-mono"
            aria-describedby={`${ids.key}-hint`}
            aria-invalid={serverErrors.key ? true : undefined}
            onChange={(event) => setKey(event.target.value)}
          />
          <p id={`${ids.key}-hint`} className="text-xs text-muted-foreground">
            {t('dialog.keyHint')}
          </p>
          {serverErrors.key && <SharedFieldError>{serverErrors.key}</SharedFieldError>}
          </>}
        </Field>
      </div>
      <Field label={t('dialog.message')} className="min-w-0">
        {(control) => <>
          <Textarea placeholder={t('dialog.messagePlaceholder')}
          {...control}
          value={message}
          rows={2}
          maxLength={MESSAGE_MAX_LENGTH}
          aria-describedby={`${ids.message}-hint`}
          aria-invalid={messageError ? true : undefined}
          onChange={(event) => setMessage(event.target.value)}
        />
        <p id={`${ids.message}-hint`} className="text-xs text-muted-foreground">
          {t('dialog.messageHint')}
        </p>
        {messageError && <SharedFieldError>{messageError}</SharedFieldError>}
        </>}
      </Field>
      <div className="grid min-w-0 gap-4 sm:grid-cols-2">
        <Field label={t('dialog.field')} className="min-w-0">
          {(control) => <>
            <NativeSelect {...control} value={field} onChange={(event) => setField(event.target.value)}>
            <option value="">{t('dialog.noField')}</option>
            {fields.map((item) => (
              <option key={item.key} value={item.key}>
                {item.label}
              </option>
            ))}
          </NativeSelect>
          <p className="text-xs text-muted-foreground">{t('dialog.fieldHint')}</p>
          {serverErrors.field && <SharedFieldError>{serverErrors.field}</SharedFieldError>}
          </>}
        </Field>
        <Field label={t('dialog.timing')} className="min-w-0">
          {(control) => <>
            <NativeSelect
            {...control}
            value={timing}
            onChange={(event) => setTiming(event.target.value as ValidationRuleTiming)}
          >
            {VALIDATION_RULE_TIMINGS.map((value) => (
              <option key={value} value={value}>
                {t(`timing.${value}`)}
              </option>
            ))}
          </NativeSelect>
          <label htmlFor={ids.active} className="mt-1 flex items-center gap-2 text-sm">
            <Checkbox id={ids.active} checked={active} onCheckedChange={(value) => setActive(value === true)} />
            {t('dialog.active')}
          </label>
          </>}
        </Field>
      </div>
      <Field id={ids.expression} label={t('dialog.expression')} className="min-w-0">
        <p id={`${ids.expression}-hint`} className="text-xs text-muted-foreground">
          {t('dialog.expressionHint')}
        </p>
        <ExpressionEditor
          id={ids.expression}
          name="expression"
          value={expression}
          errors={formula && !formula.valid ? formula.errors : []}
          invalid={Boolean(expressionError) || (formula ? !formula.valid : false)}
          describedBy={`${ids.expression}-hint ${statusId}`}
          textareaRef={(element) => {
            textareaRef.current = element
          }}
          onChange={(value) => setExpression(value.slice(0, FORMULA_EXPRESSION_MAX_LENGTH))}
        />
        {expressionError && <SharedFieldError className="break-words">{expressionError}</SharedFieldError>}
        <FormulaStatus
          id={statusId}
          preview={formula}
          expression={expression}
          validating={validating}
          failed={Boolean(request) && preview.isError && !recordMissing && !pending}
          catalog={catalog.data}
        />
        <FormulaCatalogPanel recordType={recordType} onInsert={insert} />
      </Field>
      <section className="grid min-w-0 gap-3 rounded-md border p-3" aria-labelledby={`${ids.record}-title`}>
        <h3 id={`${ids.record}-title`} className="text-sm font-medium">
          {t('test.title')}
        </h3>
        <div className="grid min-w-0 gap-3 sm:grid-cols-2">
          <Field label={t('test.record')} className="min-w-0 content-start">
            {(fieldControl) => <>
              <RecordFieldInput
              id={fieldControl.id}
              value={recordId}
              onChange={setRecordId}
              targetRecordType={recordType}
              aria-describedby={`${ids.record}-hint`}
            />
            <p id={`${ids.record}-hint`} className="text-xs text-muted-foreground">
              {t('test.recordHint')}
            </p>
            </>}
          </Field>
          <div className="flex min-h-9 min-w-0 items-center gap-2 self-end rounded-md border bg-muted/40 px-3 py-1.5 text-sm" aria-live="polite">
            {testResult}
          </div>
        </div>
      </section>
      {(generalError || serverErrors.record_type || serverErrors.applies_on) && (
        <SharedFieldError>
          {generalError ?? serverErrors.record_type ?? serverErrors.applies_on}
        </SharedFieldError>
      )}
    </DialogForm>
  )
}
