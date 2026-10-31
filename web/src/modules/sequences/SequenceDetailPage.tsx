import { Pencil, Trash2, Users } from 'lucide-react'
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
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

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
    <Page>
      <PageHeader
        back={{ to: SEQUENCES_PATH, label: t('detail.back') }}
        title={sequence.name}
        badges={<SequenceStatusBadge active={sequence.is_active} />}
        description={
          <>
            {sequence.description && <p>{sequence.description}</p>}
            <p>
              {t('detail.windowSummary', {
                start: sequence.window_start,
                end: sequence.window_end,
                limit: sequence.daily_limit,
              })}
            </p>
          </>
        }
        actions={
          canManage && (
            <>
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
            </>
          )
        }
      />
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
    </Page>
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
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead scope="col">
              {t('metrics.step')}
            </TableHead>
            <TableHead scope="col" className="text-right">
              {t('metrics.sent')}
            </TableHead>
            <TableHead scope="col" className="text-right">
              {t('metrics.opened')}
            </TableHead>
            <TableHead scope="col" className="text-right">
              {t('metrics.replied')}
            </TableHead>
            <TableHead scope="col" className="hidden text-right sm:table-cell">
              {t('metrics.bounced')}
            </TableHead>
            <TableHead scope="col" className="text-right">
              {t('metrics.exits')}
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {sequence.steps.map((step) => {
            const values = byPosition.get(step.position)
            const sent = values?.sent ?? 0
            const isEmail = step.kind === 'auto_email'
            return (
              <TableRow key={step.id}>
                <TableHead scope="row" className="py-3 text-left font-normal">
                  <span className="mr-2 text-muted-foreground tabular-nums">{step.position + 1}.</span>
                  {stepLabel(step)}
                </TableHead>
                <TableCell className="text-right tabular-nums">{step.kind === 'wait' ? '—' : sent}</TableCell>
                <TableCell className="text-right tabular-nums">
                  {isEmail ? (
                    <>
                      {values?.opened ?? 0}
                      <span className="ml-1 text-xs text-muted-foreground">{percent(values?.opened ?? 0, sent)}</span>
                    </>
                  ) : (
                    '—'
                  )}
                </TableCell>
                <TableCell className="text-right tabular-nums">
                  {isEmail ? (
                    <>
                      {values?.replied ?? 0}
                      <span className="ml-1 text-xs text-muted-foreground">{percent(values?.replied ?? 0, sent)}</span>
                    </>
                  ) : (
                    '—'
                  )}
                </TableCell>
                <TableCell className="hidden text-right tabular-nums sm:table-cell">
                  {isEmail ? (values?.bounced ?? 0) : '—'}
                </TableCell>
                <TableCell className="text-right tabular-nums">{values?.exits ?? 0}</TableCell>
              </TableRow>
            )
          })}
        </TableBody>
      </Table>
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
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">
                {t('enrollments.contact')}
              </TableHead>
              <TableHead scope="col">
                {t('enrollments.status')}
              </TableHead>
              <TableHead scope="col" className="hidden md:table-cell">
                {t('enrollments.progress')}
              </TableHead>
              <TableHead scope="col" className="hidden lg:table-cell">
                {t('enrollments.enrolledBy')}
              </TableHead>
              <TableHead scope="col" className="hidden lg:table-cell">
                {t('enrollments.nextRun')}
              </TableHead>
              {canEnroll && (
                <TableHead scope="col" className="w-24">
                  <span className="sr-only">{t('enrollments.actions')}</span>
                </TableHead>
              )}
            </TableRow>
          </TableHeader>
          <TableBody>
            {enrollments.data.data.map((enrollment) => (
              <TableRow key={enrollment.id} className="align-top">
                <TableCell>
                  <Link to={`/contacts/${encodeURIComponent(enrollment.contact_id)}`} className="font-medium hover:underline">
                    {enrollment.contact_name}
                  </Link>
                  {enrollment.contact_email && (
                    <div className="text-xs text-muted-foreground">{enrollment.contact_email}</div>
                  )}
                </TableCell>
                <TableCell>
                  <EnrollmentStatusBadge status={enrollment.status} />
                </TableCell>
                <TableCell className="hidden md:table-cell">{progress(enrollment)}</TableCell>
                <TableCell className="hidden text-muted-foreground lg:table-cell">
                  {enrollment.enrolled_by_name ?? '—'}
                  <div className="text-xs">{t(`sources.${enrollment.source}`)}</div>
                </TableCell>
                <TableCell className="hidden text-muted-foreground lg:table-cell">
                  {enrollment.next_run_at ? formatDateTime(enrollment.next_run_at, formatting) : '—'}
                </TableCell>
                {canEnroll && (
                  <TableCell>
                    <EnrollmentActions enrollment={enrollment} label={enrollment.contact_name} />
                  </TableCell>
                )}
              </TableRow>
            ))}
          </TableBody>
        </Table>
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
