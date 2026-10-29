import { zodResolver } from '@hookform/resolvers/zod'
import { useEffect, useId, useMemo } from 'react'
import { Controller, useForm, useWatch } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { DateTimeFieldInput } from '@/components/fields/DateTimeFieldInput'
import { Button } from '@/components/ui/button'
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
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast-store'
import { useOrganizationUsers } from '@/features/records/api'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useRecordFormatting } from '@/features/records/formatting'
import {
  ACTIVITY_PRIORITIES,
  useActivityTypes,
  useCreateActivity,
  type Activity,
  type ActivityPriority,
  type RecordTarget,
} from '../api'
import { activityErrorKey } from '../lib/errors'
import {
  buildRecurrenceRule,
  DEFAULT_REMINDER_MINUTES,
  EMPTY_RECURRENCE,
  recurrenceError,
} from '../lib/recurrence'
import { RecurrenceFields } from './RecurrenceFields'
import { ReminderSelect } from './ReminderSelect'
import {
  activityFormSchema,
  buildCreateRequest,
  defaultActivityType,
  type ActivityFormMode,
  type ActivityFormValues,
} from '../lib/form'

export type { ActivityFormMode } from '../lib/form'

export interface ActivityFormDefaults {
  dueAt?: string | null
  durationMinutes?: number | null
}

export interface ActivityFormDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  target?: RecordTarget
  mode?: ActivityFormMode
  defaults?: ActivityFormDefaults
  onCreated?: (activity: Activity) => void
}

function initialValues(
  activityTypeId: string,
  ownerId: string,
  defaults: ActivityFormDefaults | undefined,
): ActivityFormValues {
  return {
    title: '',
    activity_type_id: activityTypeId,
    priority: 'normal',
    owner_id: ownerId,
    due_at: defaults?.dueAt ?? null,
    completed_at: null,
    description: '',
    duration_minutes: defaults?.durationMinutes != null ? String(defaults.durationMinutes) : '',
    reminder_minutes_before: DEFAULT_REMINDER_MINUTES,
    recurrence: EMPTY_RECURRENCE,
  }
}

function FieldMessage({ id, message }: { id: string; message?: string }) {
  if (!message) return null
  return (
    <p id={id} className="text-xs text-destructive">
      {message}
    </p>
  )
}

export function ActivityFormDialog({
  open,
  onOpenChange,
  target,
  mode = 'task',
  defaults,
  onCreated,
}: ActivityFormDialogProps) {
  const { t } = useTranslation('activities')
  const { me } = useSession()
  const formatting = useRecordFormatting()
  const types = useActivityTypes()
  const users = useOrganizationUsers()
  const create = useCreateActivity(target)
  const formId = useId()
  const activeTypes = useMemo(() => types.data ?? [], [types.data])

  const defaultDueAt = defaults?.dueAt
  const defaultDuration = defaults?.durationMinutes
  const showDuration = mode === 'log' || defaultDuration != null
  const form = useForm<ActivityFormValues>({
    resolver: zodResolver(activityFormSchema),
    defaultValues: initialValues('', me.user.id, defaults),
  })
  const { register, handleSubmit, reset, control, formState, getValues, setValue } = form

  useEffect(() => {
    if (!open) return
    reset(
      initialValues(defaultActivityType(activeTypes, mode), me.user.id, {
        dueAt: defaultDueAt,
        durationMinutes: defaultDuration,
      }),
    )
  }, [open, mode, me.user.id, reset, activeTypes, defaultDueAt, defaultDuration])

  const dueAt = useWatch({ control, name: 'due_at' })
  const recurrence = useWatch({ control, name: 'recurrence' })
  const recurrenceErrorKey = mode === 'task' ? recurrenceError(recurrence, dueAt) : null

  useEffect(() => {
    if (open && !getValues('activity_type_id') && activeTypes.length > 0) {
      setValue('activity_type_id', defaultActivityType(activeTypes, mode))
    }
  }, [open, activeTypes, mode, getValues, setValue])

  const ownerOptions = useMemo(() => {
    const list = users.data ?? []
    return list.some((user) => user.id === me.user.id)
      ? list
      : [{ id: me.user.id, display_name: me.user.display_name }, ...list]
  }, [users.data, me.user.id, me.user.display_name])

  const submit = handleSubmit(async (values) => {
    if (recurrenceErrorKey) return
    try {
      const activity = await create.mutateAsync(
        buildCreateRequest(values, mode, target, mode === 'task' ? buildRecurrenceRule(recurrence) : null),
      )
      toast({ title: t(mode === 'log' ? 'form.loggedToast' : 'form.createdToast'), variant: 'success' })
      onOpenChange(false)
      onCreated?.(activity)
    } catch (error) {
      toast({ title: t(activityErrorKey(error)), variant: 'destructive' })
    }
  })

  const errors = formState.errors
  const ids = {
    title: `${formId}-title`,
    type: `${formId}-type`,
    priority: `${formId}-priority`,
    owner: `${formId}-owner`,
    when: `${formId}-when`,
    description: `${formId}-description`,
    duration: `${formId}-duration`,
    reminder: `${formId}-reminder`,
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t(mode === 'log' ? 'form.logTitle' : 'form.taskTitle')}</DialogTitle>
          <DialogDescription>{t(mode === 'log' ? 'form.logDescription' : 'form.taskDescription')}</DialogDescription>
        </DialogHeader>
        <form id={formId} className="grid gap-4" onSubmit={(event) => void submit(event)} noValidate>
          <div className="grid gap-1.5">
            <Label htmlFor={ids.title}>{t('form.title')}</Label>
            <Input
              id={ids.title}
              autoFocus
              aria-invalid={errors.title ? true : undefined}
              aria-describedby={errors.title ? `${ids.title}-error` : undefined}
              placeholder={t(mode === 'log' ? 'form.logPlaceholder' : 'form.taskPlaceholder')}
              {...register('title')}
            />
            <FieldMessage id={`${ids.title}-error`} message={errors.title ? t('form.titleRequired') : undefined} />
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="grid gap-1.5">
              <Label htmlFor={ids.type}>{t('form.type')}</Label>
              <NativeSelect id={ids.type} {...register('activity_type_id')} disabled={types.isPending}>
                {activeTypes.map((type) => (
                  <option key={type.id} value={type.id}>
                    {type.name}
                  </option>
                ))}
              </NativeSelect>
              <FieldMessage
                id={`${ids.type}-error`}
                message={errors.activity_type_id ? t('form.typeRequired') : undefined}
              />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor={ids.priority}>{t('form.priority')}</Label>
              <NativeSelect id={ids.priority} {...register('priority')}>
                {ACTIVITY_PRIORITIES.map((priority: ActivityPriority) => (
                  <option key={priority} value={priority}>
                    {t(`priority.${priority}`)}
                  </option>
                ))}
              </NativeSelect>
            </div>
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor={ids.when}>{t(mode === 'log' ? 'form.completedAt' : 'form.dueAt')}</Label>
            <Controller
              control={control}
              name={mode === 'log' ? 'completed_at' : 'due_at'}
              render={({ field }) => (
                <DateTimeFieldInput
                  id={ids.when}
                  value={field.value}
                  onChange={field.onChange}
                  onBlur={field.onBlur}
                  timeZone={formatting.timezone}
                  locale={formatting.locale}
                />
              )}
            />
            {mode === 'log' && <p className="text-xs text-muted-foreground">{t('form.completedAtHint')}</p>}
          </div>
          {mode === 'task' && (
            <div className="grid gap-1.5">
              <Label htmlFor={ids.reminder}>{t('reminder.label')}</Label>
              <Controller
                control={control}
                name="reminder_minutes_before"
                render={({ field }) => (
                  <ReminderSelect id={ids.reminder} value={field.value} onChange={field.onChange} />
                )}
              />
            </div>
          )}
          {mode === 'task' && (
            <Controller
              control={control}
              name="recurrence"
              render={({ field }) => (
                <RecurrenceFields
                  value={field.value}
                  onChange={field.onChange}
                  error={formState.isSubmitted && recurrenceErrorKey ? t(recurrenceErrorKey) : null}
                />
              )}
            />
          )}
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="grid gap-1.5">
              <Label htmlFor={ids.owner}>{t('form.owner')}</Label>
              <NativeSelect id={ids.owner} {...register('owner_id')}>
                {ownerOptions.map((user) => (
                  <option key={user.id} value={user.id}>
                    {user.id === me.user.id ? t('form.ownerMe', { name: user.display_name }) : user.display_name}
                  </option>
                ))}
              </NativeSelect>
            </div>
            {showDuration && (
              <div className="grid gap-1.5">
                <Label htmlFor={ids.duration}>{t('form.duration')}</Label>
                <Input
                  id={ids.duration}
                  inputMode="numeric"
                  aria-invalid={errors.duration_minutes ? true : undefined}
                  aria-describedby={errors.duration_minutes ? `${ids.duration}-error` : undefined}
                  {...register('duration_minutes')}
                />
                <FieldMessage
                  id={`${ids.duration}-error`}
                  message={errors.duration_minutes ? t('form.durationInvalid') : undefined}
                />
              </div>
            )}
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor={ids.description}>{t('form.description')}</Label>
            <Textarea id={ids.description} rows={3} {...register('description')} />
          </div>
        </form>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={create.isPending}>
            {t('form.cancel')}
          </Button>
          <Button type="submit" form={formId} disabled={create.isPending || types.isPending}>
            {t(mode === 'log' ? 'form.submitLog' : 'form.submitTask')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
