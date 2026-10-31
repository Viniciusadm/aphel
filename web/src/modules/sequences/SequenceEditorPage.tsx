import { zodResolver } from '@hookform/resolvers/zod'
import { ArrowDown, ArrowLeft, ArrowUp, Plus, Trash2 } from 'lucide-react'
import { useEffect, useId, useState } from 'react'
import { Controller, useFieldArray, useForm, useWatch, type Control, type FieldErrors, type UseFormRegister } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { UnsavedChangesGuard, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { useEmailTemplates } from '@/modules/email/api'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { sequencesApi, useSequence, useSequenceMutation } from './api'
import {
  MAX_DAILY_LIMIT,
  MAX_STEPS,
  MAX_WAIT_DAYS,
  SEQUENCES_MANAGE_PERMISSION,
  SEQUENCES_PATH,
  STEP_KINDS,
  sequencePath,
} from './constants'
import { sequenceErrorMessage } from './lib/errors'
import {
  defaultValues,
  emptyStep,
  isEmailKind,
  isTaskKind,
  sequenceFormSchema,
  toRequest,
  useSequencesText,
  type SequenceFormValues,
} from './lib/form'
import type { Sequence, SequenceStepKind } from './types'

const VARIABLE_EXAMPLE = '{{contact.first_name}}'
const VARIABLE_EXAMPLES = '{{contact.first_name}}, {{company.name}}, {{user.display_name}}'

export function SequenceEditorPage() {
  const { t } = useTranslation('sequences')
  const { hasPermission } = useSession()
  const { id } = useParams()
  const sequence = useSequence(id)
  if (!hasPermission(SEQUENCES_MANAGE_PERMISSION)) {
    return <EmptyState title={t('page.noAccessTitle')} description={t('editor.noAccessDescription')} />
  }
  if (id && sequence.isPending) {
    return (
      <div className="grid gap-3" aria-busy="true">
        <Skeleton className="h-10 w-64" />
        <Skeleton className="h-40 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    )
  }
  if (id && sequence.isError) {
    return <ErrorState onRetry={() => void sequence.refetch()} />
  }
  return <SequenceForm key={id ?? 'new'} sequence={id ? (sequence.data ?? null) : null} />
}

function FieldMessage({ id, message }: { id: string; message?: string }) {
  const text = useSequencesText()
  if (!message) return null
  return (
    <p id={id} className="text-sm text-destructive">
      {text(message)}
    </p>
  )
}

function SequenceForm({ sequence }: { sequence: Sequence | null }) {
  const { t } = useTranslation('sequences')
  const navigate = useNavigate()
  const formId = useId()
  const [savedId, setSavedId] = useState<string | null>(null)
  const save = useSequenceMutation((values: SequenceFormValues) =>
    sequence
      ? sequencesApi.update(sequence.id, toRequest(values), sequence.updated_at)
      : sequencesApi.create(toRequest(values)),
  )
  const {
    control,
    register,
    handleSubmit,
    setError,
    formState: { errors, isDirty, isSubmitting },
  } = useForm<SequenceFormValues>({
    resolver: zodResolver(sequenceFormSchema),
    defaultValues: defaultValues(sequence),
  })

  const onSubmit = handleSubmit(async (values) => {
    try {
      const result = await save.mutateAsync(values)
      notifySuccess(t(sequence ? 'editor.saved' : 'editor.created'))
      setSavedId(result.id)
    } catch (error) {
      if (isApiError(error) && error.code === 'sequence_key_taken') {
        setError('name', { type: 'server', message: 'errors.sequence_key_taken' })
        return
      }
      setError('root.server', { type: 'server', message: sequenceErrorMessage(t, error) })
    }
  })

  useEffect(() => {
    if (savedId) void navigate(sequencePath(savedId))
  }, [savedId, navigate])

  const backTo = sequence ? sequencePath(sequence.id) : SEQUENCES_PATH

  return (
    <div className="flex w-full max-w-4xl flex-col gap-6">
      <div className="flex flex-wrap items-center gap-3">
        <Button asChild variant="ghost" size="icon" aria-label={t('editor.back')}>
          <Link to={backTo}>
            <ArrowLeft />
          </Link>
        </Button>
        <h1 className="text-xl font-semibold">{sequence ? t('editor.titleEdit', { name: sequence.name }) : t('editor.titleNew')}</h1>
      </div>
      <form id={formId} className="grid gap-6" noValidate onSubmit={(event) => void onSubmit(event)}>
        <section className="grid gap-4 rounded-lg border p-4" aria-labelledby={`${formId}-settings`}>
          <h2 id={`${formId}-settings`} className="text-base font-medium">
            {t('editor.settings')}
          </h2>
          <div className="grid gap-1.5">
            <Label htmlFor={`${formId}-name`}>{t('editor.name')}</Label>
            <Input placeholder={t('placeholders.name')}
              id={`${formId}-name`}
              aria-invalid={errors.name ? true : undefined}
              aria-describedby={errors.name ? `${formId}-name-error` : undefined}
              {...register('name')}
            />
            <FieldMessage id={`${formId}-name-error`} message={errors.name?.message} />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor={`${formId}-description`}>{t('editor.description')}</Label>
            <Textarea placeholder={t('placeholders.description')} id={`${formId}-description`} rows={2} {...register('description')} />
            <FieldMessage id={`${formId}-description-error`} message={errors.description?.message} />
          </div>
          <Controller
            control={control}
            name="isActive"
            render={({ field }) => (
              <label className="flex items-start gap-2 text-sm">
                <Checkbox checked={field.value} onCheckedChange={(checked) => field.onChange(checked === true)} />
                <span className="grid gap-0.5">
                  <span className="font-medium">{t('editor.active')}</span>
                  <span className="text-muted-foreground">{t('editor.activeHint')}</span>
                </span>
              </label>
            )}
          />
          <div className="grid gap-4 sm:grid-cols-3">
            <div className="grid gap-1.5">
              <Label htmlFor={`${formId}-start`}>{t('editor.windowStart')}</Label>
              <Input
                id={`${formId}-start`}
                type="time"
                aria-invalid={errors.windowStart ? true : undefined}
                {...register('windowStart')}
              />
              <FieldMessage id={`${formId}-start-error`} message={errors.windowStart?.message} />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor={`${formId}-end`}>{t('editor.windowEnd')}</Label>
              <Input
                id={`${formId}-end`}
                type="time"
                aria-invalid={errors.windowEnd ? true : undefined}
                aria-describedby={errors.windowEnd ? `${formId}-end-error` : undefined}
                {...register('windowEnd')}
              />
              <FieldMessage id={`${formId}-end-error`} message={errors.windowEnd?.message} />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor={`${formId}-limit`}>{t('editor.dailyLimit')}</Label>
              <Input placeholder={t('placeholders.dailyLimit')}
                id={`${formId}-limit`}
                type="number"
                min={1}
                max={MAX_DAILY_LIMIT}
                aria-invalid={errors.dailyLimit ? true : undefined}
                aria-describedby={errors.dailyLimit ? `${formId}-limit-error` : undefined}
                {...register('dailyLimit', { valueAsNumber: true })}
              />
              <FieldMessage id={`${formId}-limit-error`} message={errors.dailyLimit?.message} />
            </div>
          </div>
          <p className="text-xs text-muted-foreground">{t('editor.windowHint')}</p>
        </section>
        <StepsEditor control={control} register={register} errors={errors} formId={formId} />
        {errors.root?.server?.message && (
          <p role="alert" className="text-sm text-destructive">
            {errors.root.server.message}
          </p>
        )}
        <div className="flex flex-wrap justify-end gap-2">
          <Button asChild variant="outline">
            <Link to={backTo}>{t('editor.cancel')}</Link>
          </Button>
          <Button type="submit" disabled={isSubmitting}>
            {isSubmitting ? t('editor.saving') : t('editor.save')}
          </Button>
        </div>
      </form>
      <UnsavedChangesGuard when={isDirty && !savedId} />
    </div>
  )
}

interface StepsEditorProps {
  control: Control<SequenceFormValues>
  register: UseFormRegister<SequenceFormValues>
  errors: FieldErrors<SequenceFormValues>
  formId: string
}

function StepsEditor({ control, register, errors, formId }: StepsEditorProps) {
  const { t } = useTranslation('sequences')
  const steps = useFieldArray({ control, name: 'steps', keyName: 'fieldKey' })
  const values = useWatch({ control, name: 'steps' })
  const templates = useEmailTemplates('organization')
  const organizationTemplates = (templates.data ?? []).filter((template) => template.scope === 'organization')

  return (
    <section className="grid gap-4" aria-labelledby={`${formId}-steps`}>
      <div className="grid gap-1">
        <h2 id={`${formId}-steps`} className="text-base font-medium">
          {t('editor.steps')}
        </h2>
        <p className="text-sm text-muted-foreground">{t('editor.stepsHint')}</p>
      </div>
      {steps.fields.length === 0 && <p className="text-sm text-muted-foreground">{t('editor.noSteps')}</p>}
      <ol className="grid gap-3">
        {steps.fields.map((field, index) => {
          const kind: SequenceStepKind = values?.[index]?.kind ?? field.kind
          const stepErrors = errors.steps?.[index]
          const prefix = `${formId}-step-${index}`
          return (
            <li key={field.fieldKey} className="grid gap-3 rounded-lg border p-4">
              <div className="flex flex-wrap items-end gap-3">
                <span className="flex size-7 shrink-0 items-center justify-center rounded-full bg-muted text-sm font-medium tabular-nums">
                  {index + 1}
                </span>
                <div className="grid min-w-48 flex-1 gap-1.5">
                  <Label htmlFor={`${prefix}-kind`}>{t('editor.kind')}</Label>
                  <NativeSelect id={`${prefix}-kind`} {...register(`steps.${index}.kind`)}>
                    {STEP_KINDS.map((option) => (
                      <option key={option} value={option}>
                        {t(`stepKinds.${option}`)}
                      </option>
                    ))}
                  </NativeSelect>
                </div>
                <div className="flex gap-1">
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    aria-label={t('editor.moveUp', { number: index + 1 })}
                    disabled={index === 0}
                    onClick={() => steps.move(index, index - 1)}
                  >
                    <ArrowUp />
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    aria-label={t('editor.moveDown', { number: index + 1 })}
                    disabled={index === steps.fields.length - 1}
                    onClick={() => steps.move(index, index + 1)}
                  >
                    <ArrowDown />
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    aria-label={t('editor.removeStep', { number: index + 1 })}
                    onClick={() => steps.remove(index)}
                  >
                    <Trash2 />
                  </Button>
                </div>
              </div>
              <p className="text-xs text-muted-foreground">{t(`stepHints.${kind}`)}</p>
              {kind === 'wait' && (
                <div className="grid max-w-48 gap-1.5">
                  <Label htmlFor={`${prefix}-wait`}>{t('editor.waitDays')}</Label>
                  <Input placeholder={t('placeholders.waitDays')}
                    id={`${prefix}-wait`}
                    type="number"
                    min={1}
                    max={MAX_WAIT_DAYS}
                    aria-invalid={stepErrors?.waitDays ? true : undefined}
                    aria-describedby={stepErrors?.waitDays ? `${prefix}-wait-error` : undefined}
                    {...register(`steps.${index}.waitDays`, { valueAsNumber: true })}
                  />
                  <FieldMessage id={`${prefix}-wait-error`} message={stepErrors?.waitDays?.message} />
                </div>
              )}
              {(isTaskKind(kind) || kind === 'manual_email') && (
                <div className="grid gap-1.5">
                  <Label htmlFor={`${prefix}-title`}>{t('editor.title')}</Label>
                  <Input
                    id={`${prefix}-title`}
                    placeholder={kind === 'manual_email' ? t('editor.titleOptional') : undefined}
                    aria-invalid={stepErrors?.title ? true : undefined}
                    aria-describedby={stepErrors?.title ? `${prefix}-title-error` : undefined}
                    {...register(`steps.${index}.title`)}
                  />
                  <FieldMessage id={`${prefix}-title-error`} message={stepErrors?.title?.message} />
                </div>
              )}
              {isEmailKind(kind) && (
                <>
                  <div className="grid gap-1.5">
                    <Label htmlFor={`${prefix}-template`}>{t('editor.template')}</Label>
                    <NativeSelect id={`${prefix}-template`} disabled={templates.isPending} {...register(`steps.${index}.templateId`)}>
                      <option value="">{t('editor.noTemplate')}</option>
                      {organizationTemplates.map((template) => (
                        <option key={template.id} value={template.id}>
                          {template.name}
                        </option>
                      ))}
                    </NativeSelect>
                    <p className="text-xs text-muted-foreground">{t('editor.templateHint')}</p>
                  </div>
                  <div className="grid gap-1.5">
                    <Label htmlFor={`${prefix}-subject`}>{t('editor.subject')}</Label>
                    <Input placeholder={t('placeholders.subject')}
                      id={`${prefix}-subject`}
                      aria-invalid={stepErrors?.subject ? true : undefined}
                      aria-describedby={stepErrors?.subject ? `${prefix}-subject-error` : `${prefix}-subject-hint`}
                      {...register(`steps.${index}.subject`)}
                    />
                    <p id={`${prefix}-subject-hint`} className="text-xs text-muted-foreground">
                      {t('editor.subjectHint', { example: VARIABLE_EXAMPLE })}
                    </p>
                    <FieldMessage id={`${prefix}-subject-error`} message={stepErrors?.subject?.message} />
                  </div>
                  <div className="grid gap-1.5">
                    <Label htmlFor={`${prefix}-body`}>{t('editor.body')}</Label>
                    <Textarea placeholder={t('placeholders.body')}
                      id={`${prefix}-body`}
                      rows={6}
                      className="font-mono text-xs"
                      aria-invalid={stepErrors?.bodyHtml ? true : undefined}
                      aria-describedby={stepErrors?.bodyHtml ? `${prefix}-body-error` : `${prefix}-body-hint`}
                      {...register(`steps.${index}.bodyHtml`)}
                    />
                    <p id={`${prefix}-body-hint`} className="text-xs text-muted-foreground">
                      {t('editor.bodyHint', { examples: VARIABLE_EXAMPLES })}
                    </p>
                    <FieldMessage id={`${prefix}-body-error`} message={stepErrors?.bodyHtml?.message} />
                  </div>
                </>
              )}
            </li>
          )
        })}
      </ol>
      <FieldMessage id={`${formId}-steps-error`} message={errors.steps?.message ?? errors.steps?.root?.message} />
      <div className="flex flex-wrap gap-2">
        <Button
          type="button"
          variant="outline"
          disabled={steps.fields.length >= MAX_STEPS}
          onClick={() => steps.append(emptyStep('auto_email'))}
        >
          <Plus />
          {t('editor.addEmail')}
        </Button>
        <Button
          type="button"
          variant="outline"
          disabled={steps.fields.length >= MAX_STEPS}
          onClick={() => steps.append(emptyStep('wait'))}
        >
          <Plus />
          {t('editor.addWait')}
        </Button>
        <Button
          type="button"
          variant="outline"
          disabled={steps.fields.length >= MAX_STEPS}
          onClick={() => steps.append(emptyStep('call'))}
        >
          <Plus />
          {t('editor.addTask')}
        </Button>
      </div>
    </section>
  )
}
