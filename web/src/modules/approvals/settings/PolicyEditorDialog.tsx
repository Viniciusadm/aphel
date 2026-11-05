import { DialogForm } from '@/components/ui/dialog-form'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast-store'
import { FilterBuilder, NativeSelect, activeFields, emptyGroup, useCustomObjects, useFieldSchema, usePipelines } from '@/sdk'
import { approvalsApi, usePolicyMutation, useQuoteRecordTypeAvailable } from '../api'
import {
  CONDITION_KINDS,
  CORE_RECORD_TYPES,
  DEAL_RECORD_TYPE,
  MAX_DUE_HOURS,
  MIN_DUE_HOURS,
  QUOTE_RECORD_TYPE,
} from '../constants'
import { approvalErrorKey } from '../errors'
import { useRecordTypeLabel } from '../lib/labels'
import type { ApprovalMoment, ApprovalPolicy, ConditionKind } from '../types'
import {
  availableMoments,
  draftFromPolicy,
  emptyDraft,
  keyFromName,
  lockableKey,
  requestFromDraft,
  serverErrors,
  validateDraft,
  type FormErrors,
  type PolicyDraft,
} from './policy-form'
import { StepsEditor } from './StepsEditor'
import { Field } from '@/components/ui/field'
import { ToggleField } from '@/components/ui/toggle-field'

const LOCAL_ERRORS = ['required', 'invalidKey', 'dueRange'] as const
type LocalError = (typeof LOCAL_ERRORS)[number]

export interface PolicyEditorDialogProps {
  open: boolean
  policy: ApprovalPolicy | null
  onOpenChange: (open: boolean) => void
}

export function PolicyEditorDialog({ open, policy, onOpenChange }: PolicyEditorDialogProps) {
  const workflow = usePipelines()
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="xl">
        {open &&
          (workflow.isPending ? (
            <Skeleton className="h-64 w-full" />
          ) : (
            <PolicyForm
              key={policy?.id ?? 'new'}
              policy={policy}
              pipelineOfStage={(stageId) =>
                workflow.data?.find((pipeline) => pipeline.stages.some((stage) => stage.id === stageId))?.id ?? ''
              }
              onOpenChange={onOpenChange}
            />
          ))}
      </DialogContent>
    </Dialog>
  )
}

interface PolicyFormProps {
  policy: ApprovalPolicy | null
  pipelineOfStage: (stageId: string) => string
  onOpenChange: (open: boolean) => void
}

function PolicyForm({ policy, pipelineOfStage, onOpenChange }: PolicyFormProps) {
  const { t } = useTranslation('approvals')
  const creating = policy === null
  const [draft, setDraft] = useState<PolicyDraft>(() =>
    policy ? draftFromPolicy(policy, pipelineOfStage) : emptyDraft(DEAL_RECORD_TYPE),
  )
  const [errors, setErrors] = useState<FormErrors>({})
  const objects = useCustomObjects()
  const quoteAvailable = useQuoteRecordTypeAvailable()
  const workflow = usePipelines()
  const schema = useFieldSchema(draft.recordType)
  const recordTypeLabel = useRecordTypeLabel()

  const save = usePolicyMutation(async (next: PolicyDraft) => {
    const request = requestFromDraft(next, schema.data, creating)
    return policy
      ? approvalsApi.updatePolicy(policy.id, request, policy.updated_at)
      : approvalsApi.createPolicy(request)
  })

  const recordTypes = useMemo(() => {
    const types: string[] = [...CORE_RECORD_TYPES]
    if (quoteAvailable.data || draft.recordType === QUOTE_RECORD_TYPE) types.push(QUOTE_RECORD_TYPE)
    for (const object of objects.data ?? []) types.push(object.record_type)
    if (!types.includes(draft.recordType)) types.push(draft.recordType)
    return types
  }, [quoteAvailable.data, objects.data, draft.recordType])

  const moments = availableMoments(draft.recordType)
  const pipeline = workflow.data?.find((item) => item.id === draft.pipelineId)
  const lockable = schema.data ? activeFields(schema.data) : []

  const patch = (next: Partial<PolicyDraft>) => setDraft((current) => ({ ...current, ...next }))

  const errorText = (value: string | undefined): string | null => {
    if (!value) return null
    return (LOCAL_ERRORS as readonly string[]).includes(value) ? t(`form.errors.${value as LocalError}`) : value
  }

  const fieldError = (field: string) => {
    const text = errorText(errors[field])
    return text ? <SharedFieldError>{text}</SharedFieldError> : null
  }

  const lockedError = Object.entries(errors).find(([field]) => field.startsWith('locked_fields'))?.[1]
  const generalErrors = Object.entries(errors).filter(
    ([field]) =>
      ![
        'name',
        'key',
        'description',
        'record_type',
        'moment',
        'target_id',
        'condition_kind',
        'condition_filter',
        'condition_formula',
        'due_hours',
      ].includes(field) &&
      !field.startsWith('locked_fields') &&
      !field.startsWith('steps.'),
  )

  const changeRecordType = (recordType: string) => {
    const allowed = availableMoments(recordType)
    patch({
      recordType,
      moment: allowed.includes(draft.moment) ? draft.moment : 'on_demand',
      pipelineId: '',
      targetId: '',
      filter: emptyGroup(),
      lockedFields: [],
    })
  }

  const submit = async () => {
    const local = validateDraft(draft, creating)
    setErrors(local)
    if (Object.keys(local).length > 0) return
    try {
      await save.mutateAsync(draft)
      toast({
        title: t(creating ? 'settings.created' : 'settings.updated'),
        variant: 'success',
      })
      onOpenChange(false)
    } catch (error) {
      if (isApiError(error) && error.code === 'validation_error') {
        const server = serverErrors(error.details)
        if (Object.keys(server).length > 0) {
          setErrors(server)
          return
        }
      }
      if (isApiError(error) && error.code === 'approval_policy_key_taken') {
        setErrors({ key: t('errors.approval_policy_key_taken') })
        return
      }
      toast({ title: t(approvalErrorKey(error)), variant: 'destructive' })
    }
  }

  return (
    <DialogForm
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
      title={creating ? t('form.createTitle') : t('form.editTitle')}
      description={t('form.description')}
      cancelLabel={t('actions.close')}
      submitLabel={t('form.save')}
      onCancel={() => onOpenChange(false)}
      pending={save.isPending}
      submitDisabled={save.isPending}
    >

      <div className="grid gap-3 sm:grid-cols-2">
        <Field label={t('form.name')}>
          {(control) => <>
            <Input placeholder={t('placeholders.name')}
            {...control}
            value={draft.name}
            maxLength={140}
            aria-invalid={errors.name ? true : undefined}
            onChange={(event) => {
              const name = event.target.value
              patch(creating && !draft.keyTouched ? { name, key: keyFromName(name) } : { name })
            }}
          />
          {fieldError('name')}
          </>}
        </Field>
        <Field label={t('form.key')}>
          {(control) => <>
            <Input placeholder={t('placeholders.key')}
            {...control}
            value={draft.key}
            maxLength={64}
            disabled={!creating}
            className="font-mono"
            aria-invalid={errors.key ? true : undefined}
            onChange={(event) => patch({ key: event.target.value, keyTouched: true })}
          />
          {creating ? fieldError('key') : <p className="text-xs text-muted-foreground">{t('form.keyLocked')}</p>}
          </>}
        </Field>
        <Field label={t('form.descriptionField')} className="sm:col-span-2">
          {(control) => <>
            <Textarea placeholder={t('placeholders.description')}
            {...control}
            value={draft.description}
            rows={2}
            maxLength={1000}
            onChange={(event) => patch({ description: event.target.value })}
          />
          {fieldError('description')}
          </>}
        </Field>
      </div>

      <div className="grid gap-3 sm:grid-cols-2">
        <Field label={t('form.recordType')}>
          {(control) => <>
            <NativeSelect
            {...control}
            value={draft.recordType}
            onChange={(event) => changeRecordType(event.target.value)}
          >
            {recordTypes.map((recordType) => (
              <option key={recordType} value={recordType}>
                {recordTypeLabel(recordType)}
              </option>
            ))}
          </NativeSelect>
          {fieldError('record_type')}
          </>}
        </Field>
        <Field label={t('form.moment')}>
          {(control) => <>
            <NativeSelect
            {...control}
            value={draft.moment}
            onChange={(event) =>
              patch({
                moment: event.target.value as ApprovalMoment,
                pipelineId: '',
                targetId: '',
              })
            }
          >
            {moments.map((moment) => (
              <option key={moment} value={moment}>
                {t(`moments.${moment}`)}
              </option>
            ))}
          </NativeSelect>
          {fieldError('moment')}
          </>}
        </Field>
        {draft.moment === 'stage_change' && (
          <>
            <Field label={t('form.pipeline')}>
              {(control) => <NativeSelect
                {...control}
                value={draft.pipelineId}
                onChange={(event) => patch({ pipelineId: event.target.value, targetId: '' })}
              >
                <option value="">{t('form.choose')}</option>
                {(workflow.data ?? []).map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.name}
                  </option>
                ))}
              </NativeSelect>}
            </Field>
            <Field label={t('form.stage')}>
              {(control) => <>
                <NativeSelect
                {...control}
                value={draft.targetId}
                disabled={!pipeline}
                aria-invalid={errors.target_id ? true : undefined}
                onChange={(event) => patch({ targetId: event.target.value })}
              >
                <option value="">{t('form.choose')}</option>
                {(pipeline?.stages ?? []).map((stage) => (
                  <option key={stage.id} value={stage.id}>
                    {stage.name}
                  </option>
                ))}
              </NativeSelect>
              {fieldError('target_id')}
              </>}
            </Field>
          </>
        )}
      </div>

      <div className="grid gap-3">
        <Field label={t('form.condition')} className="sm:max-w-xs">
          {(control) => <>
            <NativeSelect
            {...control}
            value={draft.conditionKind}
            onChange={(event) => patch({ conditionKind: event.target.value as ConditionKind })}
          >
            {CONDITION_KINDS.map((kind) => (
              <option key={kind} value={kind}>
                {t(`conditionKinds.${kind}`)}
              </option>
            ))}
          </NativeSelect>
          {fieldError('condition_kind')}
          </>}
        </Field>
        {draft.conditionKind === 'filter' &&
          (schema.data ? (
            <FilterBuilder schema={schema.data} value={draft.filter} onChange={(filter) => patch({ filter })} />
          ) : (
            <Skeleton className="h-16 w-full" />
          ))}
        {draft.conditionKind === 'filter' && fieldError('condition_filter')}
        {draft.conditionKind === 'formula' && (
          <Field label={t('form.formula')}>
            {(control) => <>
              <Textarea
              {...control}
              value={draft.formula}
              rows={3}
              spellCheck={false}
              className="font-mono text-sm"
              placeholder={t('form.formulaPlaceholder')}
              aria-invalid={errors.condition_formula ? true : undefined}
              onChange={(event) => patch({ formula: event.target.value })}
            />
            <p className="text-xs text-muted-foreground">{t('form.formulaHint')}</p>
            {fieldError('condition_formula')}
            </>}
          </Field>
        )}
      </div>

      <fieldset className="grid gap-2">
        <legend className="mb-1 text-sm font-medium">{t('form.lockedFields')}</legend>
        <p className="text-xs text-muted-foreground">{t('form.lockedFieldsHint')}</p>
        {schema.isPending ? (
          <Skeleton className="h-16 w-full" />
        ) : (
          <div className="grid max-h-48 gap-1.5 overflow-y-auto rounded-md border p-2 sm:grid-cols-2">
            {lockable.map((field) => {
              const key = lockableKey(field)
              const checked = draft.lockedFields.includes(key)
              return (
                <ToggleField key={key} label={field.label} checked={checked} onCheckedChange={(value) =>
                      patch({
                        lockedFields:
                          value === true
                            ? [...draft.lockedFields, key]
                            : draft.lockedFields.filter((item) => item !== key),
                      })
                    } />
              )
            })}
          </div>
        )}
        {lockedError && <SharedFieldError>{errorText(lockedError)}</SharedFieldError>}
      </fieldset>

      <div className="grid gap-3 sm:grid-cols-2">
        <Field label={t('form.dueHours')}>
          {(control) => <>
            <Input
            {...control}
            type="number"
            inputMode="numeric"
            min={MIN_DUE_HOURS}
            max={MAX_DUE_HOURS}
            value={draft.dueHours}
            placeholder={t('form.dueHoursPlaceholder')}
            aria-invalid={errors.due_hours ? true : undefined}
            onChange={(event) => patch({ dueHours: event.target.value })}
          />
          {fieldError('due_hours')}
          </>}
        </Field>
        <ToggleField label={t('form.active')} checked={draft.isActive} onCheckedChange={(value) => patch({ isActive: value === true })} className="sm:pt-7" />
      </div>

      <StepsEditor steps={draft.steps} errors={errors} errorText={errorText} onChange={(steps) => patch({ steps })} />

      {generalErrors.length > 0 && (
        <ul className="grid gap-1 text-xs text-destructive">
          {generalErrors.map(([field, message]) => (
            <li key={field}>{errorText(message)}</li>
          ))}
        </ul>
      )}

    </DialogForm>
  )
}
