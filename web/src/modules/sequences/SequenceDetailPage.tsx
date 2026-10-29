import { ArrowLeft, Pencil, Trash2, Users } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { sequencesApi, useSequence, useSequenceEnrollments, useSequenceMetrics, useSequenceMutation } from './api'
import { EnrollmentStatusBadge, SequenceStatusBadge } from './components/Badges'
import { EnrollmentActions } from './components/EnrollmentActions'
import { Pager } from './components/Pager'
import {
  ENROLLMENTS_PER_PAGE,
  ENROLLMENT_STATUSES,
  SEQUENCES_ENROLL_PERMISSION,
  SEQUENCES_MANAGE_PERMISSION,
  SEQUENCES_PATH,
  SEQUENCES_READ_PERMISSION,
  sequenceEditPath,
} from './constants'
import { notifySequenceError } from './lib/errors'
import { useStepLabel } from './lib/labels'
import type { Enrollment, EnrollmentStatus, Sequence } from './types'

export function SequenceDetailPage() {
  const { t } = useTranslation('sequences')
  const { hasPermission } = useSession()
  const { id = '' } = useParams()
  const sequence = useSequence(id)
  if (!hasPermission(SEQUENCES_READ_PERMISSION) && !hasPermission(SEQUENCES_MANAGE_PERMISSION)) {
    return <EmptyState title={t('page.noAccessTitle')} description={t('page.noAccessDescription')} />
  }
  if (sequence.isPending) {
    return (
      <div className="grid gap-3" aria-busy="true">
        <Skeleton className="h-10 w-64" />
        <Skeleton className="h-48 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    )
  }
  if (sequence.isError) {
    return <ErrorState onRetry={() => void sequence.refetch()} />
  }
  return <SequenceDetail sequence={sequence.data} />
}

function SequenceDetail({ sequence }: { sequence: Sequence }) {
  const { t } = useTranslation('sequences')
  const { hasPermission } = useSession()
  const navigate = useNavigate()
  const canManage = hasPermission(SEQUENCES_MANAGE_PERMISSION)
  const [deleting, setDeleting] = useState(false)
  const remove = useSequenceMutation(() => sequencesApi.remove(sequence.id))

  const confirmDelete = async () => {
    try {
      await remove.mutateAsync(undefined)
      notifySuccess(t('toast.deleted'))
      void navigate(SEQUENCES_PATH)
    } catch (error) {
      notifySequenceError(t, error)
    } finally {
      setDeleting(false)
    }
  }

  return (
    <div className="flex w-full flex-col gap-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex items-start gap-3">
          <Button asChild variant="ghost" size="icon" aria-label={t('detail.back')}>
            <Link to={SEQUENCES_PATH}>
              <ArrowLeft />
            </Link>
          </Button>
          <div className="grid gap-1">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="text-xl font-semibold">{sequence.name}</h1>
              <SequenceStatusBadge active={sequence.is_active} />
            </div>
            {sequence.description && <p className="text-sm text-muted-foreground">{sequence.description}</p>}
            <p className="text-sm text-muted-foreground">
              {t('detail.windowSummary', {
                start: sequence.window_start,
                end: sequence.window_end,
                limit: sequence.daily_limit,
              })}
            </p>
          </div>
        </div>
        {canManage && (
          <div className="flex gap-2">
            <Button asChild variant="outline">
              <Link to={sequenceEditPath(sequence.id)}>
                <Pencil aria-hidden="true" />
                {t('detail.edit')}
              </Link>
            </Button>
            <Button variant="outline" onClick={() => setDeleting(true)}>
              <Trash2 aria-hidden="true" />
              {t('detail.delete')}
            </Button>
          </div>
        )}
      </div>
      <dl className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        {(['active', 'paused', 'completed', 'exited'] as const).map((status) => (
          <div key={status} className="rounded-lg border p-3">
            <dt className="text-xs text-muted-foreground">{t(`enrollmentStatus.${status}`)}</dt>
            <dd className="text-xl font-semibold tabular-nums">{sequence.enrollments[status]}</dd>
          </div>
        ))}
      </dl>
      <StepMetricsSection sequence={sequence} />
      <EnrollmentsSection sequence={sequence} />
      {deleting && (
        <ConfirmDialog
          open
          variant="destructive"
          title={t('confirmDelete.title', { name: sequence.name })}
          description={t('confirmDelete.description')}
          confirmLabel={t('detail.delete')}
          pending={remove.isPending}
          onConfirm={confirmDelete}
          onOpenChange={(open) => !open && setDeleting(false)}
        />
      )}
    </div>
  )
}

function percent(part: number, total: number): string {
  if (total === 0) return ''
  return `${Math.round((part / total) * 100)}%`
}

function StepMetricsSection({ sequence }: { sequence: Sequence }) {
  const { t } = useTranslation('sequences')
  const metrics = useSequenceMetrics(sequence.id)
  const stepLabel = useStepLabel()
  const headingId = useId()

  let body
  if (sequence.steps.length === 0) {
    body = <p className="text-sm text-muted-foreground">{t('editor.noSteps')}</p>
  } else if (metrics.isPending) {
    body = <Skeleton className="h-32 w-full" />
  } else if (metrics.isError) {
    body = <ErrorState onRetry={() => void metrics.refetch()} />
  } else {
    const byPosition = new Map(metrics.data.steps.map((step) => [step.position, step]))
    body = (
      <div className="relative overflow-x-auto rounded-lg border">
        <table className="w-full text-sm">
          <thead className="bg-muted/50 text-left text-muted-foreground">
            <tr>
              <th scope="col" className="px-4 py-2 font-medium">
                {t('metrics.step')}
              </th>
              <th scope="col" className="px-4 py-2 text-right font-medium">
                {t('metrics.sent')}
              </th>
              <th scope="col" className="px-4 py-2 text-right font-medium">
                {t('metrics.opened')}
              </th>
              <th scope="col" className="px-4 py-2 text-right font-medium">
                {t('metrics.replied')}
              </th>
              <th scope="col" className="hidden px-4 py-2 text-right font-medium sm:table-cell">
                {t('metrics.bounced')}
              </th>
              <th scope="col" className="px-4 py-2 text-right font-medium">
                {t('metrics.exits')}
              </th>
            </tr>
          </thead>
          <tbody>
            {sequence.steps.map((step) => {
              const values = byPosition.get(step.position)
              const sent = values?.sent ?? 0
              const isEmail = step.kind === 'auto_email'
              return (
                <tr key={step.id} className="border-t">
                  <th scope="row" className="px-4 py-3 text-left font-normal">
                    <span className="mr-2 text-muted-foreground tabular-nums">{step.position + 1}.</span>
                    {stepLabel(step)}
                  </th>
                  <td className="px-4 py-3 text-right tabular-nums">{step.kind === 'wait' ? '—' : sent}</td>
                  <td className="px-4 py-3 text-right tabular-nums">
                    {isEmail ? (
                      <>
                        {values?.opened ?? 0}
                        <span className="ml-1 text-xs text-muted-foreground">{percent(values?.opened ?? 0, sent)}</span>
                      </>
                    ) : (
                      '—'
                    )}
                  </td>
                  <td className="px-4 py-3 text-right tabular-nums">
                    {isEmail ? (
                      <>
                        {values?.replied ?? 0}
                        <span className="ml-1 text-xs text-muted-foreground">{percent(values?.replied ?? 0, sent)}</span>
                      </>
                    ) : (
                      '—'
                    )}
                  </td>
                  <td className="hidden px-4 py-3 text-right tabular-nums sm:table-cell">
                    {isEmail ? (values?.bounced ?? 0) : '—'}
                  </td>
                  <td className="px-4 py-3 text-right tabular-nums">{values?.exits ?? 0}</td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>
    )
  }

  return (
    <section className="grid gap-3" aria-labelledby={headingId}>
      <div className="grid gap-1">
        <h2 id={headingId} className="text-base font-medium">
          {t('metrics.title')}
        </h2>
        <p className="text-sm text-muted-foreground">{t('metrics.description')}</p>
      </div>
      {body}
    </section>
  )
}

function EnrollmentsSection({ sequence }: { sequence: Sequence }) {
  const { t } = useTranslation('sequences')
  const { hasPermission } = useSession()
  const formatting = useRecordFormatting()
  const headingId = useId()
  const statusId = useId()
  const [status, setStatus] = useState<EnrollmentStatus | ''>('')
  const [page, setPage] = useState(1)
  const enrollments = useSequenceEnrollments(sequence.id, { page, status: status || undefined })
  const canEnroll = hasPermission(SEQUENCES_ENROLL_PERMISSION)
  const stepLabel = useStepLabel()

  const progress = (enrollment: Enrollment) => {
    if (enrollment.status === 'completed') return t('enrollments.finished')
    if (enrollment.status === 'exited') {
      return enrollment.exit_reason ? t(`exitReasons.${enrollment.exit_reason}`) : t('enrollmentStatus.exited')
    }
    const step = sequence.steps[enrollment.current_position]
    return step
      ? t('enrollments.nextStep', { number: enrollment.current_position + 1, step: stepLabel(step) })
      : t('enrollments.finished')
  }

  let body
  if (enrollments.isPending) {
    body = <Skeleton className="h-32 w-full" />
  } else if (enrollments.isError) {
    body = <ErrorState onRetry={() => void enrollments.refetch()} />
  } else if (enrollments.data.data.length === 0) {
    body = <EmptyState icon={Users} title={t('enrollments.emptyTitle')} description={t('enrollments.emptyDescription')} />
  } else {
    body = (
      <>
        <div className="relative overflow-x-auto rounded-lg border">
          <table className="w-full text-sm">
            <thead className="bg-muted/50 text-left text-muted-foreground">
              <tr>
                <th scope="col" className="px-4 py-2 font-medium">
                  {t('enrollments.contact')}
                </th>
                <th scope="col" className="px-4 py-2 font-medium">
                  {t('enrollments.status')}
                </th>
                <th scope="col" className="hidden px-4 py-2 font-medium md:table-cell">
                  {t('enrollments.progress')}
                </th>
                <th scope="col" className="hidden px-4 py-2 font-medium lg:table-cell">
                  {t('enrollments.enrolledBy')}
                </th>
                <th scope="col" className="hidden px-4 py-2 font-medium lg:table-cell">
                  {t('enrollments.nextRun')}
                </th>
                {canEnroll && (
                  <th scope="col" className="w-24 px-4 py-2">
                    <span className="sr-only">{t('enrollments.actions')}</span>
                  </th>
                )}
              </tr>
            </thead>
            <tbody>
              {enrollments.data.data.map((enrollment) => (
                <tr key={enrollment.id} className="border-t align-top">
                  <td className="px-4 py-3">
                    <Link to={`/contacts/${encodeURIComponent(enrollment.contact_id)}`} className="font-medium hover:underline">
                      {enrollment.contact_name}
                    </Link>
                    {enrollment.contact_email && (
                      <div className="text-xs text-muted-foreground">{enrollment.contact_email}</div>
                    )}
                  </td>
                  <td className="px-4 py-3">
                    <EnrollmentStatusBadge status={enrollment.status} />
                  </td>
                  <td className="hidden px-4 py-3 md:table-cell">{progress(enrollment)}</td>
                  <td className="hidden px-4 py-3 text-muted-foreground lg:table-cell">
                    {enrollment.enrolled_by_name ?? '—'}
                    <div className="text-xs">{t(`sources.${enrollment.source}`)}</div>
                  </td>
                  <td className="hidden px-4 py-3 text-muted-foreground lg:table-cell">
                    {enrollment.next_run_at ? formatDateTime(enrollment.next_run_at, formatting) : '—'}
                  </td>
                  {canEnroll && (
                    <td className="px-4 py-3">
                      <EnrollmentActions enrollment={enrollment} label={enrollment.contact_name} />
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <Pager page={page} perPage={ENROLLMENTS_PER_PAGE} total={enrollments.data.meta.total} onPageChange={setPage} />
      </>
    )
  }

  return (
    <section className="grid gap-3" aria-labelledby={headingId}>
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div className="grid gap-1">
          <h2 id={headingId} className="text-base font-medium">
            {t('enrollments.title')}
          </h2>
          <p className="text-sm text-muted-foreground">{t('enrollments.description')}</p>
        </div>
        <div className="grid w-48 gap-1.5">
          <Label htmlFor={statusId} className="sr-only">
            {t('enrollments.statusFilter')}
          </Label>
          <NativeSelect
            id={statusId}
            value={status}
            onChange={(event) => {
              setStatus(event.target.value as EnrollmentStatus | '')
              setPage(1)
            }}
          >
            <option value="">{t('enrollments.allStatuses')}</option>
            {ENROLLMENT_STATUSES.map((option) => (
              <option key={option} value={option}>
                {t(`enrollmentStatus.${option}`)}
              </option>
            ))}
          </NativeSelect>
        </div>
      </div>
      {body}
    </section>
  )
}
