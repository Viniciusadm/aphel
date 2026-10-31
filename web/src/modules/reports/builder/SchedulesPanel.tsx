import { CalendarClock, Pencil, Plus, Trash2 } from 'lucide-react'
import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { useOrganizationUsers } from '@/features/records/api'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import {
  useReportSchedules,
  useScheduleDeliveries,
  useScheduleMutations,
  type PeriodPreset,
  type Report,
  type ReportSchedule,
  type ReportScheduleFrequency,
} from './api'
import { PERIOD_PRESETS } from './model'
import { Section } from './BuilderSections'

const FREQUENCIES: ReportScheduleFrequency[] = ['daily', 'weekly', 'monthly']
const WEEKDAYS = [1, 2, 3, 4, 5, 6, 7]

interface ScheduleValues {
  frequency: ReportScheduleFrequency
  weekday: number
  dayOfMonth: number
  time: string
  periodPreset: PeriodPreset | ''
  recipients: string[]
  isActive: boolean
}

function initialValues(schedule: ReportSchedule | null, currentUserId: string): ScheduleValues {
  if (!schedule) {
    return {
      frequency: 'weekly',
      weekday: 1,
      dayOfMonth: 1,
      time: '08:00',
      periodPreset: '',
      recipients: [currentUserId],
      isActive: true,
    }
  }
  return {
    frequency: schedule.frequency,
    weekday: schedule.weekday ?? 1,
    dayOfMonth: schedule.day_of_month ?? 1,
    time: `${String(schedule.hour).padStart(2, '0')}:${String(schedule.minute).padStart(2, '0')}`,
    periodPreset: (schedule.period_preset as PeriodPreset | null | undefined) ?? '',
    recipients: schedule.recipients.map((recipient) => recipient.id),
    isActive: schedule.is_active,
  }
}

function scheduleRequest(values: ScheduleValues) {
  const [hour, minute] = values.time.split(':').map(Number)
  return {
    frequency: values.frequency,
    weekday: values.frequency === 'weekly' ? values.weekday : null,
    day_of_month: values.frequency === 'monthly' ? values.dayOfMonth : null,
    hour: Number.isFinite(hour) ? hour : 8,
    minute: Number.isFinite(minute) ? minute : 0,
    period_preset: values.periodPreset || null,
    recipients: values.recipients,
    is_active: values.isActive,
  }
}

function ScheduleForm({
  report,
  schedule,
  currentUserId,
  pending,
  onCancel,
  onSubmit,
}: {
  report: Report
  schedule: ReportSchedule | null
  currentUserId: string
  pending: boolean
  onCancel: () => void
  onSubmit: (values: ScheduleValues) => void
}) {
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })
  const { t: tCommon } = useTranslation()
  const ids = {
    frequency: useId(),
    weekday: useId(),
    day: useId(),
    time: useId(),
    preset: useId(),
    search: useId(),
    active: useId(),
  }
  const users = useOrganizationUsers()
  const [values, setValues] = useState(() => initialValues(schedule, currentUserId))
  const [search, setSearch] = useState('')
  const [submitted, setSubmitted] = useState(false)
  const set = (update: Partial<ScheduleValues>) => setValues((current) => ({ ...current, ...update }))
  const hasPeriod = Boolean(report.definition.period)
  const filtered = (users.data ?? []).filter((user) =>
    user.display_name.toLowerCase().includes(search.trim().toLowerCase()),
  )
  const recipientsError = values.recipients.length === 0 ? t('schedules.form.recipientsRequired') : null

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault()
    setSubmitted(true)
    if (recipientsError || !values.time) return
    onSubmit(values)
  }

  return (
    <form className="grid gap-4" onSubmit={handleSubmit} noValidate>
      <div className="grid gap-3 sm:grid-cols-2">
        <div className="grid gap-2.5">
          <Label htmlFor={ids.frequency}>{t('schedules.form.frequency')}</Label>
          <NativeSelect
            id={ids.frequency}
            value={values.frequency}
            onChange={(event) => set({ frequency: event.target.value as ReportScheduleFrequency })}
          >
            {FREQUENCIES.map((frequency) => (
              <option key={frequency} value={frequency}>
                {t(`schedules.frequency.${frequency}`)}
              </option>
            ))}
          </NativeSelect>
        </div>
        {values.frequency === 'weekly' && (
          <div className="grid gap-2.5">
            <Label htmlFor={ids.weekday}>{t('schedules.form.weekday')}</Label>
            <NativeSelect
              id={ids.weekday}
              value={values.weekday}
              onChange={(event) => set({ weekday: Number(event.target.value) })}
            >
              {WEEKDAYS.map((weekday) => (
                <option key={weekday} value={weekday}>
                  {t(`schedules.weekdays.${weekday}` as 'schedules.weekdays.1')}
                </option>
              ))}
            </NativeSelect>
          </div>
        )}
        {values.frequency === 'monthly' && (
          <div className="grid gap-2.5">
            <Label htmlFor={ids.day}>{t('schedules.form.dayOfMonth')}</Label>
            <Input placeholder={t('placeholders.day')}
              id={ids.day}
              type="number"
              min={1}
              max={31}
              value={values.dayOfMonth}
              onChange={(event) => set({ dayOfMonth: Math.min(31, Math.max(1, Math.round(Number(event.target.value)) || 1)) })}
            />
          </div>
        )}
        <div className="grid gap-2.5">
          <Label htmlFor={ids.time}>{t('schedules.form.time')}</Label>
          <Input id={ids.time} type="time" value={values.time} onChange={(event) => set({ time: event.target.value })} />
        </div>
        {hasPeriod && (
          <div className="grid gap-2.5">
            <Label htmlFor={ids.preset}>{t('schedules.form.period')}</Label>
            <NativeSelect
              id={ids.preset}
              value={values.periodPreset}
              onChange={(event) => set({ periodPreset: event.target.value as PeriodPreset | '' })}
            >
              <option value="">{t('schedules.form.periodAsSaved')}</option>
              {PERIOD_PRESETS.filter((preset) => preset !== 'custom').map((preset) => (
                <option key={preset} value={preset}>
                  {t(`presets.${preset}`)}
                </option>
              ))}
            </NativeSelect>
          </div>
        )}
      </div>
      <fieldset className="grid gap-2">
        <legend className="mb-1 text-sm font-medium">{t('schedules.form.recipients')}</legend>
        <p className="text-xs text-muted-foreground">{t('schedules.form.recipientsHint')}</p>
        <Input
          id={ids.search}
          aria-label={t('schedules.form.searchUsers')}
          placeholder={t('schedules.form.searchUsers')}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
        <div className="grid max-h-48 gap-1 overflow-y-auto rounded-md border p-2">
          {users.isPending && <Skeleton className="h-6 w-full" />}
          {filtered.map((user) => {
            const checked = values.recipients.includes(user.id)
            const id = `${ids.search}-${user.id}`
            return (
              <div key={user.id} className="flex items-center gap-2">
                <Checkbox
                  id={id}
                  checked={checked}
                  onCheckedChange={(next) =>
                    set({
                      recipients:
                        next === true
                          ? [...values.recipients, user.id]
                          : values.recipients.filter((recipient) => recipient !== user.id),
                    })
                  }
                />
                <Label htmlFor={id} className="font-normal">
                  {user.display_name}
                </Label>
              </div>
            )
          })}
          {users.isSuccess && filtered.length === 0 && (
            <p className="text-sm text-muted-foreground">{t('schedules.form.noUsers')}</p>
          )}
        </div>
        {submitted && recipientsError && <p className="text-sm text-destructive">{recipientsError}</p>}
      </fieldset>
      <div className="flex items-center gap-2">
        <Checkbox id={ids.active} checked={values.isActive} onCheckedChange={(checked) => set({ isActive: checked === true })} />
        <Label htmlFor={ids.active}>{t('schedules.form.active')}</Label>
      </div>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onCancel}>
          {tCommon('actions.cancel')}
        </Button>
        <Button type="submit" disabled={pending}>
          {pending ? t('common.saving') : t('common.save')}
        </Button>
      </DialogFooter>
    </form>
  )
}

function Deliveries({ reportId, scheduleId }: { reportId: string; scheduleId: string }) {
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })
  const formatting = useRecordFormatting()
  const deliveries = useScheduleDeliveries(reportId, scheduleId)
  if (deliveries.isPending) return <Skeleton className="h-6 w-full" />
  if (deliveries.isError) return <p className="text-sm text-destructive">{t('schedules.deliveries.error')}</p>
  if (deliveries.data.data.length === 0) {
    return <p className="text-sm text-muted-foreground">{t('schedules.deliveries.empty')}</p>
  }
  return (
    <ul className="grid gap-1 text-sm">
      {deliveries.data.data.map((delivery) => (
        <li key={delivery.id} className="flex flex-wrap items-center gap-x-2 gap-y-0.5">
          <Badge variant={delivery.status === 'sent' ? 'success' : delivery.status === 'sending' ? 'secondary' : 'destructive'}>
            {t(`schedules.deliveries.status.${delivery.status}`)}
          </Badge>
          <span>{delivery.recipient?.display_name ?? delivery.user_id}</span>
          <span className="text-muted-foreground">{formatDateTime(delivery.scheduled_for, formatting)}</span>
          {delivery.failure_reason && (
            <span className="basis-full text-xs text-muted-foreground">{delivery.failure_reason}</span>
          )}
        </li>
      ))}
    </ul>
  )
}

function describeSchedule(schedule: ReportSchedule, t: ReturnType<typeof useTranslation<'reports', 'builder'>>['t']) {
  const time = `${String(schedule.hour).padStart(2, '0')}:${String(schedule.minute).padStart(2, '0')}`
  if (schedule.frequency === 'weekly') {
    return t('schedules.summary.weekly', {
      weekday: t(`schedules.weekdays.${schedule.weekday ?? 1}` as 'schedules.weekdays.1'),
      time,
    })
  }
  if (schedule.frequency === 'monthly') {
    return t('schedules.summary.monthly', { day: schedule.day_of_month ?? 1, time })
  }
  return t('schedules.summary.daily', { time })
}

export function SchedulesPanel({ report, currentUserId }: { report: Report; currentUserId: string }) {
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })
  const formatting = useRecordFormatting()
  const schedules = useReportSchedules(report.id)
  const mutations = useScheduleMutations(report.id)
  const [editing, setEditing] = useState<ReportSchedule | null>(null)
  const [formOpen, setFormOpen] = useState(false)
  const [removing, setRemoving] = useState<ReportSchedule | null>(null)
  const [expanded, setExpanded] = useState<string | null>(null)
  const pending = mutations.create.isPending || mutations.update.isPending

  const openForm = (schedule: ReportSchedule | null) => {
    setEditing(schedule)
    setFormOpen(true)
  }

  const submit = (values: ScheduleValues) => {
    const request = scheduleRequest(values)
    const done = {
      onSuccess: () => {
        setFormOpen(false)
        notifySuccess(t('schedules.saved'))
      },
      onError: (error: unknown) => notifyError(error),
    }
    if (editing) mutations.update.mutate({ schedule: editing, request }, done)
    else mutations.create.mutate(request, done)
  }

  return (
    <Section title={t('schedules.title')} description={t('schedules.description')}>
      {schedules.isPending && <Skeleton className="h-10 w-full" />}
      {schedules.isError && <p className="text-sm text-destructive">{t('schedules.loadError')}</p>}
      {schedules.isSuccess && schedules.data.length === 0 && (
        <p className="text-sm text-muted-foreground">{t('schedules.empty')}</p>
      )}
      {schedules.isSuccess && schedules.data.length > 0 && (
        <ul className="grid gap-2">
          {schedules.data.map((schedule) => (
            <li key={schedule.id} className="grid gap-2 rounded-md border p-3">
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div className="grid min-w-0 gap-0.5">
                  <p className="flex flex-wrap items-center gap-2 text-sm font-medium">
                    <CalendarClock className="size-4 text-muted-foreground" aria-hidden />
                    {describeSchedule(schedule, t)}
                    {!schedule.is_active && <Badge variant="outline">{t('schedules.inactive')}</Badge>}
                  </p>
                  <p className="text-xs text-muted-foreground">
                    {t('schedules.recipientsList', {
                      names: schedule.recipients.map((recipient) => recipient.display_name).join(', '),
                    })}
                  </p>
                  {schedule.is_active && (
                    <p className="text-xs text-muted-foreground">
                      {t('schedules.nextRun', { date: formatDateTime(schedule.next_run_at, formatting) })}
                    </p>
                  )}
                </div>
                <div className="flex gap-1">
                  <Button type="button" variant="ghost" size="icon" aria-label={t('schedules.edit')} onClick={() => openForm(schedule)}>
                    <Pencil />
                  </Button>
                  <Button type="button" variant="ghost" size="icon" aria-label={t('schedules.remove')} onClick={() => setRemoving(schedule)}>
                    <Trash2 />
                  </Button>
                </div>
              </div>
              <div>
                <Button
                  type="button"
                  variant="link"
                  size="sm"
                  className="h-auto px-0"
                  aria-expanded={expanded === schedule.id}
                  onClick={() => setExpanded((current) => (current === schedule.id ? null : schedule.id))}
                >
                  {expanded === schedule.id ? t('schedules.deliveries.hide') : t('schedules.deliveries.show')}
                </Button>
                {expanded === schedule.id && <Deliveries reportId={report.id} scheduleId={schedule.id} />}
              </div>
            </li>
          ))}
        </ul>
      )}
      <div>
        <Button type="button" variant="outline" size="sm" onClick={() => openForm(null)}>
          <Plus />
          {t('schedules.add')}
        </Button>
      </div>
      <Dialog open={formOpen} onOpenChange={setFormOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{editing ? t('schedules.editTitle') : t('schedules.createTitle')}</DialogTitle>
            <DialogDescription>{t('schedules.formDescription')}</DialogDescription>
          </DialogHeader>
          {formOpen && (
            <ScheduleForm
              key={editing?.id ?? 'new'}
              report={report}
              schedule={editing}
              currentUserId={currentUserId}
              pending={pending}
              onCancel={() => setFormOpen(false)}
              onSubmit={submit}
            />
          )}
        </DialogContent>
      </Dialog>
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        title={t('schedules.removeTitle')}
        description={t('schedules.removeDescription')}
        confirmLabel={t('schedules.remove')}
        variant="destructive"
        pending={mutations.remove.isPending}
        onConfirm={async () => {
          if (!removing) return
          await mutations.remove.mutateAsync(removing.id)
          setRemoving(null)
          notifySuccess(t('schedules.removed'))
        }}
      />
    </Section>
  )
}
