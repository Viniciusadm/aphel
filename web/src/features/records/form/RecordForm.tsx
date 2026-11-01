import { zodResolver } from '@hookform/resolvers/zod'
import { Loader2 } from 'lucide-react'
import { useEffect, useId, useMemo, useRef, type ReactNode } from 'react'
import { Controller, useForm, useWatch, type FieldErrors } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { DialogFooter } from '@/components/ui/dialog'
import { cn } from '@/lib/utils'
import { isReadOnlyField, type RecordPayload, type RecordValues } from '../fields'
import type { FieldDefinition, FieldSchema, RecordItem } from '../types'
import { RecordFieldControl } from './RecordFieldControl'
import { UnsavedChangesGuard } from '@/components/feedback'
import { mapRecordApiError } from './lib/errors'
import { useFilters } from '@/kernel/modules/filters'
import { Slot } from '@/kernel/modules/Slot'
import type { FormLayout, ResolvedLayoutSection } from './lib/layout'
import { resolveLayout, resolvedFields } from './lib/layout'
import { buildRecordSchema, isBlank } from './lib/validation'
import {
  READ_ONLY_SYSTEM_KEYS,
  buildSubmitPayload,
  initialFormValues,
  resolveFormFields,
  type RecordFormLayout,
} from './lib/values'
import { layoutRuleStates, recordRuleValues, useConditionContext } from './use-form-rules'
import { Alert } from '@/components/ui/alert'

export type RecordFormMode = 'create' | 'edit'

export interface RecordFormProps {
  schema?: FieldSchema
  fields?: FieldDefinition[]
  layout?: RecordFormLayout
  formLayout?: FormLayout | null
  readOnlyKeys?: string[]
  record?: RecordItem
  defaultValues?: RecordValues
  mode?: RecordFormMode
  onSubmit: (payload: RecordPayload, values: RecordValues) => Promise<unknown> | unknown
  onCancel?: () => void
  onDirtyChange?: (dirty: boolean) => void
  submitLabel?: string
  columns?: 1 | 2
  guardUnsavedChanges?: boolean
  disabled?: boolean
  id?: string
  className?: string
}

const SERVER_ERROR_PATH = 'root.server'
const WIDE_FIELD_TYPES = new Set(['long_text', 'multi_select'])
const SECTION_COLUMNS: Record<number, string> = {
  2: 'sm:grid-cols-2',
  3: 'sm:grid-cols-2 lg:grid-cols-3',
}

export function RecordForm({
  schema,
  fields,
  layout,
  formLayout,
  readOnlyKeys = READ_ONLY_SYSTEM_KEYS,
  record,
  defaultValues,
  mode = record ? 'edit' : 'create',
  onSubmit,
  onCancel,
  onDirtyChange,
  submitLabel,
  columns = 2,
  guardUnsavedChanges = true,
  disabled = false,
  id,
  className,
}: RecordFormProps) {
  const { t } = useTranslation('records-form')
  const generatedId = useId()
  const formId = id ?? generatedId
  const knownFields = useMemo(() => schema?.fields ?? fields ?? [], [schema, fields])
  const recordType = schema?.record_type
  const layoutContext = useMemo(() => ({ recordType, mode }), [recordType, mode])
  const effectiveLayout = useFilters('records.formLayout', formLayout ?? null, layoutContext)
  const resolved = useMemo(() => {
    if (!effectiveLayout) return null
    const candidates = fields
      ? resolveFormFields(fields, { readOnlyKeys: [] })
      : schema
        ? resolveFormFields(schema, { readOnlyKeys })
        : []
    return resolveLayout(effectiveLayout, candidates, { appendRequired: mode === 'create' })
  }, [effectiveLayout, fields, schema, readOnlyKeys, mode])
  const formFields = useMemo(() => {
    if (resolved) return resolvedFields(resolved).map((item) => item.field)
    if (fields) return resolveFormFields(fields, { layout, readOnlyKeys: [] })
    return schema ? resolveFormFields(schema, { layout, readOnlyKeys }) : []
  }, [resolved, fields, schema, layout, readOnlyKeys])
  const initialValues = useMemo(
    () => initialFormValues(formFields, { record, defaultValues }),
    [formFields, record, defaultValues],
  )
  const validationSchema = useMemo(() => buildRecordSchema(formFields, t, initialValues), [formFields, t, initialValues])
  const submittingRef = useRef(false)
  const conditionContext = useConditionContext(knownFields)
  const baseRuleValues = useMemo(
    () => recordRuleValues(knownFields, record, defaultValues),
    [knownFields, record, defaultValues],
  )

  const {
    control,
    handleSubmit,
    reset,
    setError,
    setFocus,
    formState: { errors, isDirty, isSubmitting },
  } = useForm<RecordValues, unknown, RecordValues>({
    resolver: zodResolver(validationSchema),
    values: initialValues,
    resetOptions: { keepDirtyValues: true },
  })
  const watched = useWatch({ control }) as RecordValues
  const ruleStates = useMemo(
    () => layoutRuleStates(resolved, { ...baseRuleValues, ...watched }, conditionContext),
    [resolved, baseRuleValues, watched, conditionContext],
  )

  useEffect(() => {
    onDirtyChange?.(isDirty)
  }, [isDirty, onDirtyChange])

  const revealField = (key: string | undefined) => {
    if (!key) return
    setFocus(key)
    document.getElementById(`${formId}-${key}`)?.scrollIntoView({ block: 'center', behavior: 'smooth' })
  }

  const onInvalid = (invalid: FieldErrors<RecordValues>) => {
    revealField(formFields.find((field) => invalid[field.key])?.key)
  }

  const onValid = async (values: RecordValues) => {
    const missing = formFields.filter((field) => ruleStates[field.key]?.required && isBlank(values[field.key]))
    if (missing.length > 0) {
      missing.forEach((field, index) =>
        setError(field.key, { type: 'required', message: t('validation.required') }, { shouldFocus: index === 0 }),
      )
      revealField(missing[0].key)
      return
    }
    const submitted = formFields.filter(
      (field) => ruleStates[field.key]?.visible !== false && !isReadOnlyField(field),
    )
    const payload = buildSubmitPayload(submitted, values, mode, initialValues)
    if (mode === 'edit' && Object.keys(payload).length === 0) {
      reset(values)
      return
    }
    submittingRef.current = true
    try {
      await onSubmit(payload, values)
      reset(values)
    } catch (error) {
      const mapped = mapRecordApiError(
        error,
        formFields.map((field) => field.key),
        t,
        knownFields,
      )
      mapped.fields.forEach((item, index) =>
        setError(item.key, { type: 'server', message: item.message }, { shouldFocus: index === 0 }),
      )
      revealField(mapped.fields[0]?.key)
      if (mapped.general) setError(SERVER_ERROR_PATH, { type: 'server', message: mapped.general })
    } finally {
      submittingRef.current = false
    }
  }

  const serverError = errors.root?.server?.message
  const busy = disabled || isSubmitting
  const label = isSubmitting
    ? t(mode === 'create' ? 'actions.creating' : 'actions.saving')
    : (submitLabel ?? t(mode === 'create' ? 'actions.create' : 'actions.save'))

  const renderField = (field: FieldDefinition, wide?: string | false) => {
    const state = ruleStates[field.key]
    if (state && !state.visible) return null
    const displayed = state?.required && !field.is_required ? { ...field, is_required: true } : field
    return (
      <Controller
        key={field.key}
        control={control}
        name={field.key}
        render={({ field: controller, fieldState }) => (
          <RecordFieldControl
            field={displayed}
            id={`${formId}-${field.key}`}
            value={controller.value}
            onChange={controller.onChange}
            onBlur={controller.onBlur}
            initialValue={initialValues[field.key]}
            error={fieldState.error?.message}
            disabled={busy || isReadOnlyField(field)}
            className={cn(wide)}
          />
        )}
      />
    )
  }

  const renderSection = (section: ResolvedLayoutSection) => (
    <fieldset key={section.id} className="grid min-w-0 gap-3" data-section={section.id}>
      {section.label && <legend className="mb-3 text-sm font-semibold">{section.label}</legend>}
      <div className={cn('grid min-w-0 gap-4', SECTION_COLUMNS[section.columns.length])}>
        {section.columns.map((column, index) => (
          <div key={index} className="grid min-w-0 content-start gap-4">
            {column.fields.map((item) => renderField(item.field))}
          </div>
        ))}
      </div>
    </fieldset>
  )

  let body: ReactNode
  if (formFields.length === 0) {
    body = <p className="text-sm text-muted-foreground">{t('form.noFields')}</p>
  } else if (resolved && resolved.tabs.length > 1) {
    body = <div className="grid min-w-0 gap-8">
      <nav aria-label={t('layout.tabs')} className="flex flex-wrap gap-x-4 gap-y-2 border-b pb-4">
        {resolved.tabs.map((tab, index) => <a key={tab.id} href={`#${formId}-section-${tab.id}`} className="inline-flex min-h-11 items-center text-sm text-muted-foreground underline-offset-4 hover:text-foreground hover:underline">{tab.label || t('layout.untitledTab', { number: index + 1 })}</a>)}
      </nav>
      {resolved.tabs.map((tab, index) => <section key={tab.id} id={`${formId}-section-${tab.id}`} className="grid min-w-0 scroll-mt-20 gap-5">
        <h2 className="text-base font-semibold">{tab.label || t('layout.untitledTab', { number: index + 1 })}</h2>
        {tab.sections.map(renderSection)}
      </section>)}
    </div>
  } else if (resolved) {
    body = <div className="grid min-w-0 gap-6">{resolved.tabs[0]?.sections.map(renderSection)}</div>
  } else {
    body = (
      <div className={cn('grid min-w-0 gap-4', columns === 2 && 'sm:grid-cols-2')}>
        {formFields.map((field) =>
          renderField(field, columns === 2 && WIDE_FIELD_TYPES.has(field.field_type) && 'sm:col-span-2'),
        )}
      </div>
    )
  }

  return (
    <>
      <form
        id={formId}
        noValidate
        onSubmit={(event) => void handleSubmit(onValid, onInvalid)(event)}
        className={cn('grid min-w-0 gap-4', className)}
      >
        {serverError && (
          <Alert className="text-destructive">
            {serverError}
          </Alert>
        )}
        {body}
        <Slot name="record.form.sections" props={{ recordType, mode, record }} />
        <DialogFooter>
          {onCancel && (
            <Button variant="outline" onClick={onCancel} disabled={isSubmitting}>
              {t('actions.cancel')}
            </Button>
          )}
          <Button type="submit" disabled={busy || formFields.length === 0 || (mode === 'edit' && !isDirty)}>
            {isSubmitting && <Loader2 className="animate-spin" aria-hidden="true" />}
            {label}
          </Button>
        </DialogFooter>
      </form>
      {guardUnsavedChanges && <UnsavedChangesGuard when={() => isDirty && !submittingRef.current} />}
    </>
  )
}
