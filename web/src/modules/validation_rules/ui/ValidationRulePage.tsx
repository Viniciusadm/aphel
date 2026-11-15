import { Loader2, ListChecks } from 'lucide-react'
import { useMemo, useRef, useState } from 'react'
import { useNavigate, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { RecordFieldInput } from '@/components/fields'
import { notifyError, notifySuccess, PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Textarea } from '@/components/ui/textarea'
import { ToggleField } from '@/components/ui/toggle-field'
import {
  activeFields,
  ExpressionEditor,
  FORMULA_EXPRESSION_MAX_LENGTH,
  FORMULA_PREVIEW_DEBOUNCE_MS,
  FormulaCatalogPanel,
  FormulaStatus,
  fieldViolations,
  NativeSelect,
  RequirePermission,
  SETTINGS_MANAGE,
  SettingsCard,
  useDebounced,
  useFieldSchema,
  useFormulaCatalog,
  WIDE_FIELD,
  type FormulaSnippet,
} from '@/sdk'
import {
  useInvalidateValidationRules,
  useValidationRulePreview,
  useValidationRules,
  validationRulesApi,
} from '@/modules/validation_rules/ui/api'
import { VALIDATION_RULES_PATH } from '@/modules/validation_rules/ui/routes'
import {
  VALIDATION_RULE_TIMINGS,
  type ValidationRule,
  type ValidationRulePreviewRequest,
  type ValidationRuleTiming,
} from '@/modules/validation_rules/ui/types'

const NAME_MAX_LENGTH = 120
const MESSAGE_MAX_LENGTH = 500
const FORM_ID = 'validation-rule-form'

export function ValidationRulePage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <RuleLoader />
    </RequirePermission>
  )
}

function RuleLoader() {
  const { t } = useTranslation('validation-rules')
  const { recordType = '', ruleId = '' } = useParams()
  const rules = useValidationRules(recordType)
  const back = { to: `${VALIDATION_RULES_PATH}/${recordType}`, label: t('edit.back') }

  if (ruleId === 'new') return <RuleEditor key="new" recordType={recordType} rule={null} />
  if (rules.isPending)
    return (
      <Page>
        <PageHeader title={t('title')} back={back} />
        <PageLoader />
      </Page>
    )
  if (rules.isError)
    return (
      <Page>
        <PageHeader title={t('title')} back={back} />
        <ErrorState onRetry={() => void rules.refetch()} />
      </Page>
    )
  const rule = rules.data.find((item) => item.id === ruleId)
  if (!rule)
    return (
      <Page>
        <PageHeader title={t('title')} back={back} />
        <EmptyState icon={ListChecks} title={t('edit.notFoundTitle')} description={t('edit.notFoundDescription')} />
      </Page>
    )
  return <RuleEditor key={rule.updated_at} recordType={recordType} rule={rule} />
}

function RuleEditor({ recordType, rule }: { recordType: string; rule: ValidationRule | null }) {
  const { t } = useTranslation('validation-rules')
  const navigate = useNavigate()
  const invalidate = useInvalidateValidationRules()
  const schema = useFieldSchema(recordType)
  const catalog = useFormulaCatalog(recordType)
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)
  const initial = useMemo(
    () => ({
      name: rule?.name ?? '',
      message: rule?.message ?? '',
      field: rule?.field ?? '',
      timing: (rule?.applies_on ?? 'always') as ValidationRuleTiming,
      active: rule?.is_active ?? true,
      expression: rule?.expression ?? '',
    }),
    [rule],
  )
  const [values, setValues] = useState(initial)
  const [recordId, setRecordId] = useState<string | null>(null)
  const [submitted, setSubmitted] = useState(false)
  const [saving, setSaving] = useState(false)
  const [serverErrors, setServerErrors] = useState<Record<string, string>>({})
  const dirty = JSON.stringify(values) !== JSON.stringify(initial)
  const patch = (next: Partial<typeof values>) => setValues((current) => ({ ...current, ...next }))
  const back = `${VALIDATION_RULES_PATH}/${recordType}`

  const fields = schema.data ? activeFields(schema.data) : []
  const trimmed = values.expression.trim()
  const request: ValidationRulePreviewRequest | null = trimmed
    ? { record_type: recordType, expression: values.expression, ...(recordId ? { record_id: recordId } : {}) }
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
  const statusId = 'validation-rule-expression-status'

  const nameError = submitted && !values.name.trim() ? t('dialog.required') : serverErrors.name
  const messageError = submitted && !values.message.trim() ? t('dialog.required') : serverErrors.message
  const expressionError = submitted && !trimmed ? t('dialog.required') : serverErrors.expression

  const insert = ({ text, caret }: FormulaSnippet) => {
    const element = textareaRef.current
    const start = element?.selectionStart ?? values.expression.length
    const end = element?.selectionEnd ?? values.expression.length
    patch({ expression: `${values.expression.slice(0, start)}${text}${values.expression.slice(end)}` })
    const position = start + (caret ?? text.length)
    requestAnimationFrame(() => {
      element?.focus()
      element?.setSelectionRange(position, position)
    })
  }

  const submit = async () => {
    setSubmitted(true)
    if (!values.name.trim() || !values.message.trim() || !trimmed) return
    setSaving(true)
    setServerErrors({})
    const body = {
      name: values.name.trim(),
      message: values.message.trim(),
      expression: values.expression,
      field: values.field || null,
      applies_on: values.timing,
      is_active: values.active,
    }
    try {
      if (rule) {
        await validationRulesApi.update(rule.id, body, rule.updated_at)
        await invalidate()
        notifySuccess(t('toasts.updated'))
      } else {
        await validationRulesApi.create({ record_type: recordType, ...body })
        await invalidate()
        notifySuccess(t('toasts.created'))
        void navigate(back)
      }
    } catch (error) {
      if (isApiError(error) && error.status === 409) {
        notifyError(error, { title: t('toasts.conflict') })
      } else {
        const violations = fieldViolations(isApiError(error) ? error.details : null)
        const mapped: Record<string, string> = {}
        for (const violation of violations) mapped[violation.field] ??= violation.message
        setServerErrors(mapped)
        if (violations.length === 0) notifyError(error)
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
    <Page>
      <UnsavedChangesGuard when={dirty && !saving} />
      <PageHeader
        title={rule ? rule.name : t('dialog.newTitle')}
        description={t('edit.description')}
        back={{ to: back, label: t('edit.back') }}
        actions={
          <Button type="submit" form={FORM_ID} disabled={saving || (Boolean(rule) && !dirty)}>
            {t('dialog.save')}
          </Button>
        }
      />
      <form
        id={FORM_ID}
        className="grid gap-6"
        noValidate
        onSubmit={(event) => {
          event.preventDefault()
          void submit()
        }}
      >
        <SettingsCard title={t('edit.ruleTitle')} description={t('edit.ruleDescription')} disabled={saving}>
          <Field label={t('dialog.name')} error={nameError}>
            {(control) => (
              <Input
                {...control}
                value={values.name}
                maxLength={NAME_MAX_LENGTH}
                placeholder={t('dialog.namePlaceholder')}
                onChange={(event) => patch({ name: event.target.value })}
              />
            )}
          </Field>
          <Field label={t('dialog.timing')} error={serverErrors.applies_on}>
            {(control) => (
              <NativeSelect
                {...control}
                value={values.timing}
                onChange={(event) => patch({ timing: event.target.value as ValidationRuleTiming })}
              >
                {VALIDATION_RULE_TIMINGS.map((value) => (
                  <option key={value} value={value}>
                    {t(`timing.${value}`)}
                  </option>
                ))}
              </NativeSelect>
            )}
          </Field>
          <Field label={t('dialog.message')} description={t('dialog.messageHint')} error={messageError} className={WIDE_FIELD}>
            {(control) => (
              <Textarea
                {...control}
                rows={2}
                value={values.message}
                maxLength={MESSAGE_MAX_LENGTH}
                placeholder={t('dialog.messagePlaceholder')}
                onChange={(event) => patch({ message: event.target.value })}
              />
            )}
          </Field>
          <Field label={t('dialog.field')} description={t('dialog.fieldHint')} error={serverErrors.field}>
            {(control) => (
              <NativeSelect {...control} value={values.field} onChange={(event) => patch({ field: event.target.value })}>
                <option value="">{t('dialog.noField')}</option>
                {fields.map((item) => (
                  <option key={item.key} value={item.key}>
                    {item.label}
                  </option>
                ))}
              </NativeSelect>
            )}
          </Field>
          <ToggleField
            type="switch"
            label={t('dialog.active')}
            description={t('dialog.activeHint')}
            checked={values.active}
            onCheckedChange={(active) => patch({ active })}
          />
        </SettingsCard>

        <SettingsCard title={t('edit.conditionTitle')} description={t('help')} disabled={saving}>
          <Field id="validation-rule-expression" label={t('dialog.expression')} hideLabel className={WIDE_FIELD}>
            <p id="validation-rule-expression-hint" className="text-sm text-muted-foreground">
              {t('dialog.expressionHint')}
            </p>
            <ExpressionEditor
              id="validation-rule-expression"
              name="expression"
              value={values.expression}
              errors={formula && !formula.valid ? formula.errors : []}
              invalid={Boolean(expressionError) || (formula ? !formula.valid : false)}
              describedBy={`validation-rule-expression-hint ${statusId}`}
              textareaRef={(element) => {
                textareaRef.current = element
              }}
              onChange={(value) => patch({ expression: value.slice(0, FORMULA_EXPRESSION_MAX_LENGTH) })}
            />
            {expressionError && <p className="text-sm break-words text-destructive">{expressionError}</p>}
            <FormulaStatus
              id={statusId}
              preview={formula}
              expression={values.expression}
              validating={validating}
              failed={Boolean(request) && preview.isError && !recordMissing && !pending}
              catalog={catalog.data}
            />
            <FormulaCatalogPanel recordType={recordType} onInsert={insert} />
          </Field>
        </SettingsCard>

        <SettingsCard title={t('test.title')} description={t('test.recordHint')} disabled={saving}>
          <Field label={t('test.record')}>
            {(control) => (
              <RecordFieldInput id={control.id} value={recordId} onChange={setRecordId} targetRecordType={recordType} />
            )}
          </Field>
          <div className="flex min-h-9 min-w-0 items-center gap-2 self-end rounded-md border bg-muted/40 px-3 py-1.5 text-sm" aria-live="polite">
            {testResult}
          </div>
        </SettingsCard>
      </form>
    </Page>
  )
}
