import { zodResolver } from '@hookform/resolvers/zod'
import { useId, useMemo } from 'react'
import { useForm, useWatch } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { isApiError } from '@/api'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { fieldViolations } from '@/features/auth/lib/api-errors'
import { FormRow } from '@/features/settings/organization/components/FormRow'
import { NativeSelect } from '@/features/settings/organization/components/NativeSelect'
import {
  GOAL_METRICS,
  GOAL_PERIODS,
  goalCreditMode,
  goalsApi,
  isCountMetric,
  useGoalOwners,
  type Goal,
  type GoalBody,
  type GoalMetric,
  type GoalOwnerOptions,
  type GoalOwnerType,
  type GoalPeriod,
} from './api'
import { CreditModeField } from './CreditModeField'
import { useGoalPeriodLabel } from './format'

export interface GoalDialogProps {
  open: boolean
  editing: Goal | null
  onOpenChange: (open: boolean) => void
  onSaved: (goal: Goal) => void
}

export function GoalDialog({ open, editing, onOpenChange, onSaved }: GoalDialogProps) {
  const { t } = useTranslation('goals')
  const owners = useGoalOwners()
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{t(editing ? 'dialog.editTitle' : 'dialog.newTitle')}</DialogTitle>
          <DialogDescription>{t('dialog.description')}</DialogDescription>
        </DialogHeader>
        {open &&
          (owners.isPending ? (
            <Skeleton className="h-64 w-full" />
          ) : owners.isError ? (
            <ErrorState onRetry={() => void owners.refetch()} />
          ) : (
            <GoalForm
              key={editing?.id ?? 'new'}
              owners={owners.data}
              editing={editing}
              onClose={() => onOpenChange(false)}
              onSaved={onSaved}
            />
          ))}
      </DialogContent>
    </Dialog>
  )
}

const schema = z
  .object({
    ownerType: z.enum(['user', 'team', 'organization']),
    userId: z.string(),
    teamId: z.string(),
    metric: z.enum(['won_amount', 'won_count', 'activities_done', 'new_contacts']),
    period: z.enum(['month', 'quarter', 'year']),
    month: z.string().regex(/^\d{4}-\d{2}$/, 'dialog.errors.month'),
    target: z.string().trim(),
    creditMode: z.enum(['owner', 'split']),
  })
  .superRefine((values, context) => {
    if (values.ownerType === 'user' && !values.userId) {
      context.addIssue({ code: 'custom', path: ['userId'], message: 'dialog.errors.user' })
    }
    if (values.ownerType === 'team' && !values.teamId) {
      context.addIssue({ code: 'custom', path: ['teamId'], message: 'dialog.errors.team' })
    }
    const target = Number(values.target.replace(',', '.'))
    if (!values.target || !Number.isFinite(target) || target <= 0) {
      context.addIssue({ code: 'custom', path: ['target'], message: 'dialog.errors.target' })
    } else if (isCountMetric(values.metric) && !Number.isInteger(target)) {
      context.addIssue({ code: 'custom', path: ['target'], message: 'dialog.errors.targetInteger' })
    }
  })

type FormValues = z.infer<typeof schema>

function currentMonth(): string {
  const now = new Date()
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`
}

function availableOwnerTypes(owners: GoalOwnerOptions): GoalOwnerType[] {
  const types: GoalOwnerType[] = []
  if (owners.users.length > 0) types.push('user')
  if (owners.teams.length > 0) types.push('team')
  if (owners.organization) types.push('organization')
  return types
}

function initialValues(editing: Goal | null, owners: GoalOwnerOptions): FormValues {
  if (editing) {
    return {
      ownerType: editing.owner_type,
      userId: editing.user_id ?? '',
      teamId: editing.team_id ?? '',
      metric: editing.metric,
      period: editing.period,
      month: editing.starts_on.slice(0, 7),
      target: editing.target,
      creditMode: goalCreditMode(editing),
    }
  }
  return {
    ownerType: availableOwnerTypes(owners)[0] ?? 'user',
    userId: owners.users.length === 1 ? owners.users[0].id : '',
    teamId: owners.teams.length === 1 ? owners.teams[0].id : '',
    metric: 'won_amount',
    period: 'month',
    month: currentMonth(),
    target: '',
    creditMode: 'owner',
  }
}

function periodStart(period: GoalPeriod, month: string): string | null {
  if (!/^\d{4}-\d{2}$/.test(month)) return null
  const [year, value] = month.split('-').map(Number)
  if (period === 'year') return `${year}-01-01`
  if (period === 'quarter') return `${year}-${String(Math.floor((value - 1) / 3) * 3 + 1).padStart(2, '0')}-01`
  return `${month}-01`
}

const serverFields: Record<string, keyof FormValues> = {
  owner_type: 'ownerType',
  user_id: 'userId',
  team_id: 'teamId',
  metric: 'metric',
  period: 'period',
  starts_on: 'month',
  target: 'target',
}

interface GoalFormProps {
  owners: GoalOwnerOptions
  editing: Goal | null
  onClose: () => void
  onSaved: (goal: Goal) => void
}

function GoalForm({ owners, editing, onClose, onSaved }: GoalFormProps) {
  const { t } = useTranslation('goals')
  const formId = useId()
  const periodLabel = useGoalPeriodLabel()
  const {
    register,
    handleSubmit,
    setError,
    control,
    formState: { errors, isSubmitting },
  } = useForm<FormValues>({ resolver: zodResolver(schema), defaultValues: initialValues(editing, owners) })
  const ownerType = useWatch({ control, name: 'ownerType' })
  const period = useWatch({ control, name: 'period' })
  const month = useWatch({ control, name: 'month' })
  const metric = useWatch({ control, name: 'metric' })

  const ownerTypes = useMemo(() => {
    const types = availableOwnerTypes(owners)
    if (editing && !types.includes(editing.owner_type)) types.push(editing.owner_type)
    return types
  }, [owners, editing])
  const users = useMemo(() => {
    if (!editing?.user_id || owners.users.some((user) => user.id === editing.user_id)) return owners.users
    return [...owners.users, { id: editing.user_id, name: editing.owner_name ?? editing.user_id }]
  }, [owners.users, editing])
  const teams = useMemo(() => {
    if (!editing?.team_id || owners.teams.some((team) => team.id === editing.team_id)) return owners.teams
    return [...owners.teams, { id: editing.team_id, name: editing.owner_name ?? editing.team_id }]
  }, [owners.teams, editing])

  const start = periodStart(period, month)
  const errorText = (message?: string) => (message ? t(message as 'dialog.errors.target') : undefined)

  const onSubmit = handleSubmit(async (values) => {
    const startsOn = periodStart(values.period, values.month)
    if (!startsOn) return
    const body: GoalBody = {
      owner_type: values.ownerType,
      user_id: values.ownerType === 'user' ? values.userId : null,
      team_id: values.ownerType === 'team' ? values.teamId : null,
      metric: values.metric,
      period: values.period,
      starts_on: startsOn,
      target: values.target.replace(',', '.'),
      credit_mode: values.creditMode,
    }
    try {
      const saved = editing ? await goalsApi.update(editing.id, body, editing.updated_at) : await goalsApi.create(body)
      notifySuccess(t(editing ? 'toast.updated' : 'toast.created'))
      onSaved(saved)
      onClose()
    } catch (error) {
      if (isApiError(error) && error.code === 'goal_already_exists') {
        setError('period', { message: 'dialog.errors.duplicate' })
        return
      }
      if (isApiError(error) && error.code === 'version_conflict') {
        notifyError(error, { title: t('dialog.errors.conflict') })
        return
      }
      const violations = isApiError(error) ? fieldViolations(error.details) : []
      let handled = false
      for (const violation of violations) {
        const field = serverFields[violation.field]
        if (field) {
          setError(field, { message: 'dialog.errors.invalid' })
          handled = true
        }
      }
      if (!handled) notifyError(error)
    }
  })

  return (
    <>
      <form id={formId} className="grid min-w-0 gap-4" noValidate onSubmit={(event) => void onSubmit(event)}>
        <fieldset className="grid min-w-0 gap-4 sm:grid-cols-2" disabled={isSubmitting}>
          <FormRow id={`${formId}-owner-type`} label={t('dialog.ownerType')} error={errorText(errors.ownerType?.message)}>
            {(describedBy) => (
              <NativeSelect id={`${formId}-owner-type`} aria-describedby={describedBy} {...register('ownerType')}>
                {ownerTypes.map((type) => (
                  <option key={type} value={type}>
                    {t(`ownerTypes.${type}`)}
                  </option>
                ))}
              </NativeSelect>
            )}
          </FormRow>
          {ownerType === 'user' && (
            <FormRow id={`${formId}-user`} label={t('dialog.user')} error={errorText(errors.userId?.message)}>
              {(describedBy) => (
                <NativeSelect
                  id={`${formId}-user`}
                  aria-describedby={describedBy}
                  aria-invalid={errors.userId ? true : undefined}
                  {...register('userId')}
                >
                  <option value="">{t('dialog.choose')}</option>
                  {users.map((user) => (
                    <option key={user.id} value={user.id}>
                      {user.name}
                    </option>
                  ))}
                </NativeSelect>
              )}
            </FormRow>
          )}
          {ownerType === 'team' && (
            <FormRow id={`${formId}-team`} label={t('dialog.team')} error={errorText(errors.teamId?.message)}>
              {(describedBy) => (
                <NativeSelect
                  id={`${formId}-team`}
                  aria-describedby={describedBy}
                  aria-invalid={errors.teamId ? true : undefined}
                  {...register('teamId')}
                >
                  <option value="">{t('dialog.choose')}</option>
                  {teams.map((team) => (
                    <option key={team.id} value={team.id}>
                      {team.name}
                    </option>
                  ))}
                </NativeSelect>
              )}
            </FormRow>
          )}
          <FormRow id={`${formId}-metric`} label={t('dialog.metric')} error={errorText(errors.metric?.message)}>
            {(describedBy) => (
              <NativeSelect id={`${formId}-metric`} aria-describedby={describedBy} {...register('metric')}>
                {GOAL_METRICS.map((value: GoalMetric) => (
                  <option key={value} value={value}>
                    {t(`metrics.${value}`)}
                  </option>
                ))}
              </NativeSelect>
            )}
          </FormRow>
          <FormRow id={`${formId}-period`} label={t('dialog.period')} error={errorText(errors.period?.message)}>
            {(describedBy) => (
              <NativeSelect
                id={`${formId}-period`}
                aria-describedby={describedBy}
                aria-invalid={errors.period ? true : undefined}
                {...register('period')}
              >
                {GOAL_PERIODS.map((value) => (
                  <option key={value} value={value}>
                    {t(`periods.${value}`)}
                  </option>
                ))}
              </NativeSelect>
            )}
          </FormRow>
          <FormRow
            id={`${formId}-month`}
            label={t('dialog.month')}
            error={errorText(errors.month?.message)}
            hint={start ? t('dialog.periodHint', { period: periodLabel({ period, starts_on: start }) }) : undefined}
          >
            {(describedBy) => (
              <Input
                id={`${formId}-month`}
                type="month"
                aria-describedby={describedBy}
                aria-invalid={errors.month ? true : undefined}
                {...register('month')}
              />
            )}
          </FormRow>
          <FormRow
            id={`${formId}-target`}
            label={t(isCountMetric(metric) ? 'dialog.targetCount' : 'dialog.targetAmount')}
            error={errorText(errors.target?.message)}
          >
            {(describedBy) => (
              <Input placeholder={t('placeholders.target')}
                id={`${formId}-target`}
                inputMode="decimal"
                autoComplete="off"
                aria-describedby={describedBy}
                aria-invalid={errors.target ? true : undefined}
                {...register('target')}
              />
            )}
          </FormRow>
          {metric === 'won_amount' && ownerType !== 'organization' && (
            <CreditModeField id={`${formId}-credit-mode`} className="sm:col-span-2" {...register('creditMode')} />
          )}
        </fieldset>
      </form>
      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={isSubmitting}>
          {t('dialog.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={isSubmitting}>
          {t('dialog.save')}
        </Button>
      </DialogFooter>
    </>
  )
}
