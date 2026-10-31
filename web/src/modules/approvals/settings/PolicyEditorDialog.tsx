import { useId, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast-store'
import { useCustomObjects } from '@/features/custom-records'
import { usePipelines } from '@/modules/pipelines/api'
import { useFieldSchema } from '@/features/records/api'
import { activeFields } from '@/features/records/fields'
import { FilterBuilder, emptyGroup } from '@/features/records/filters'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
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

const LOCAL_ERRORS = ['required', 'invalidKey', 'dueRange'] as const
type LocalError = (typeof LOCAL_ERRORS)[number]

export interface PolicyEditorDialogProps {
  open: boolean
  policy: ApprovalPolicy | null
  onOpenChange: (open: boolean) => void
}

export function PolicyEditorDialog({ open, policy, onOpenChange }: PolicyEditorDialogProps) {
  const pipelines = usePipelines()
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="xl">
        {open &&
          (pipelines.isPending ? (
            <Skeleton className="h-64 w-full" />
          ) : (
            <PolicyForm
              key={policy?.id ?? 'new'}
              policy={policy}
              pipelineOfStage={(stageId) =>
                pipelines.data?.find((pipeline) => pipeline.stages.some((stage) => stage.id === stageId))?.id ?? ''
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
  const ids = {
    name: useId(),
    key: useId(),
    description: useId(),
    recordType: useId(),
    moment: useId(),
    pipeline: useId(),
    stage: useId(),
    condition: useId(),
    formula: useId(),
    due: useId(),
    active: useId(),
  }
  const creating = policy === null
  const [draft, setDraft] = useState<PolicyDraft>(() =>
    policy ? draftFromPolicy(policy, pipelineOfStage) : emptyDraft(DEAL_RECORD_TYPE),
  )
  const [errors, setErrors] = useState<FormErrors>({})
  const objects = useCustomObjects()
  const quoteAvailable = useQuoteRecordTypeAvailable()
  const pipelines = usePipelines()
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
  const pipeline = pipelines.data?.find((item) => item.id === draft.pipelineId)
  const lockable = schema.data ? activeFields(schema.data) : []

  const patch = (next: Partial<PolicyDraft>) => setDraft((current) => ({ ...current, ...next }))

  const errorText = (value: string | undefined): string | null => {
    if (!value) return null
    return (LOCAL_ERRORS as readonly string[]).includes(value) ? t(`form.errors.${value as LocalError}`) : value
  }

  const fieldError = (field: string) => {
    const text = errorText(errors[field])
    return text ? <p className="text-xs text-destructive">{text}</p> : null
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
    <form
      className="grid gap-5"
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
    >
      <DialogHeader>
        <DialogTitle>{creating ? t('form.createTitle') : t('form.editTitle')}</DialogTitle>
        <DialogDescription>{t('form.description')}</DialogDescription>
      </DialogHeader>

      <div className="grid gap-3 sm:grid-cols-2">
        <div className="grid gap-2.5">
          <Label htmlFor={ids.name}>{t('form.name')}</Label>
          <Input placeholder={t('placeholders.name')}
            id={ids.name}
            value={draft.name}
            maxLength={140}
            aria-invalid={errors.name ? true : undefined}
            onChange={(event) => {
              const name = event.target.value
              patch(creating && !draft.keyTouched ? { name, key: keyFromName(name) } : { name })
            }}
          />
          {fieldError('name')}
        </div>
        <div className="grid gap-2.5">
          <Label htmlFor={ids.key}>{t('form.key')}</Label>
          <Input placeholder={t('placeholders.key')}
            id={ids.key}
            value={draft.key}
            maxLength={64}
            disabled={!creating}
            className="font-mono"
            aria-invalid={errors.key ? true : undefined}
            onChange={(event) => patch({ key: event.target.value, keyTouched: true })}
          />
          {creating ? fieldError('key') : <p className="text-xs text-muted-foreground">{t('form.keyLocked')}</p>}
        </div>
        <div className="grid gap-2.5 sm:col-span-2">
          <Label htmlFor={ids.description}>{t('form.descriptionField')}</Label>
          <Textarea placeholder={t('placeholders.description')}
            id={ids.description}
            value={draft.description}
            rows={2}
            maxLength={1000}
            onChange={(event) => patch({ description: event.target.value })}
          />
          {fieldError('description')}
        </div>
      </div>

      <div className="grid gap-3 sm:grid-cols-2">
        <div className="grid gap-2.5">
          <Label htmlFor={ids.recordType}>{t('form.recordType')}</Label>
          <NativeSelect
            id={ids.recordType}
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
        </div>
        <div className="grid gap-2.5">
          <Label htmlFor={ids.moment}>{t('form.moment')}</Label>
          <NativeSelect
            id={ids.moment}
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
        </div>
        {draft.moment === 'stage_change' && (
          <>
            <div className="grid gap-2.5">
              <Label htmlFor={ids.pipeline}>{t('form.pipeline')}</Label>
              <NativeSelect
                id={ids.pipeline}
                value={draft.pipelineId}
                onChange={(event) => patch({ pipelineId: event.target.value, targetId: '' })}
              >
                <option value="">{t('form.choose')}</option>
                {(pipelines.data ?? []).map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.name}
                  </option>
                ))}
              </NativeSelect>
            </div>
            <div className="grid gap-2.5">
              <Label htmlFor={ids.stage}>{t('form.stage')}</Label>
              <NativeSelect
                id={ids.stage}
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
            </div>
          </>
        )}
      </div>

      <div className="grid gap-3">
        <div className="grid gap-2.5 sm:max-w-xs">
          <Label htmlFor={ids.condition}>{t('form.condition')}</Label>
          <NativeSelect
            id={ids.condition}
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
        </div>
        {draft.conditionKind === 'filter' &&
          (schema.data ? (
            <FilterBuilder schema={schema.data} value={draft.filter} onChange={(filter) => patch({ filter })} />
          ) : (
            <Skeleton className="h-16 w-full" />
          ))}
        {draft.conditionKind === 'filter' && fieldError('condition_filter')}
        {draft.conditionKind === 'formula' && (
          <div className="grid gap-2.5">
            <Label htmlFor={ids.formula}>{t('form.formula')}</Label>
            <Textarea
              id={ids.formula}
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
          </div>
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
              const checkboxId = `${ids.recordType}-lock-${key}`
              return (
                <div key={key} className="flex items-center gap-2">
                  <Checkbox
                    id={checkboxId}
                    checked={checked}
                    onCheckedChange={(value) =>
                      patch({
                        lockedFields:
                          value === true
                            ? [...draft.lockedFields, key]
                            : draft.lockedFields.filter((item) => item !== key),
                      })
                    }
                  />
                  <Label htmlFor={checkboxId} className="text-sm font-normal">
                    {field.label}
                  </Label>
                </div>
              )
            })}
          </div>
        )}
        {lockedError && <p className="text-xs text-destructive">{errorText(lockedError)}</p>}
      </fieldset>

      <div className="grid gap-3 sm:grid-cols-2">
        <div className="grid gap-2.5">
          <Label htmlFor={ids.due}>{t('form.dueHours')}</Label>
          <Input
            id={ids.due}
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
        </div>
        <div className="flex items-center gap-2 sm:pt-7">
          <Checkbox
            id={ids.active}
            checked={draft.isActive}
            onCheckedChange={(value) => patch({ isActive: value === true })}
          />
          <Label htmlFor={ids.active} className="font-normal">
            {t('form.active')}
          </Label>
        </div>
      </div>

      <StepsEditor steps={draft.steps} errors={errors} errorText={errorText} onChange={(steps) => patch({ steps })} />

      {generalErrors.length > 0 && (
        <ul className="grid gap-1 text-xs text-destructive">
          {generalErrors.map(([field, message]) => (
            <li key={field}>{errorText(message)}</li>
          ))}
        </ul>
      )}

      <DialogFooter>
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={save.isPending}>
          {t('actions.close')}
        </Button>
        <Button type="submit" disabled={save.isPending}>
          {t('form.save')}
        </Button>
      </DialogFooter>
    </form>
  )
}
