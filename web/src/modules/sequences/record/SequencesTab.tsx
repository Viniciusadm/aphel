import { ListOrdered, Plus } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import { useContactEnrollments } from '../api'
import { EnrollmentStatusBadge } from '../components/Badges'
import { EnrollmentActions } from '../components/EnrollmentActions'
import { SEQUENCES_ENROLL_PERMISSION, sequencePath } from '../constants'
import type { Enrollment } from '../types'
import { EnrollDialog } from './EnrollDialog'

export interface SequencesTabProps {
  contactId: string
  readOnly?: boolean
}

export function SequencesTab({ contactId, readOnly = false }: SequencesTabProps) {
  const { t } = useTranslation('sequences')
  const { hasPermission } = useSession()
  const formatting = useRecordFormatting()
  const enrollments = useContactEnrollments(contactId)
  const [enrolling, setEnrolling] = useState(false)
  const canEnroll = !readOnly && hasPermission(SEQUENCES_ENROLL_PERMISSION)
  const live = (enrollments.data ?? []).filter((item) => item.status === 'active' || item.status === 'paused')

  const detail = (enrollment: Enrollment) => {
    if (enrollment.status === 'exited') {
      const reason = enrollment.exit_reason ? t(`exitReasons.${enrollment.exit_reason}`) : ''
      const at = enrollment.exited_at ? formatDateTime(enrollment.exited_at, formatting) : ''
      return [reason, at].filter(Boolean).join(' · ')
    }
    if (enrollment.status === 'completed') {
      return enrollment.completed_at ? t('record.completedAt', { date: formatDateTime(enrollment.completed_at, formatting) }) : ''
    }
    if (enrollment.status === 'paused') return t('record.pausedDetail', { number: enrollment.current_position + 1 })
    return enrollment.next_run_at
      ? t('record.nextRun', { number: enrollment.current_position + 1, date: formatDateTime(enrollment.next_run_at, formatting) })
      : t('record.stepNumber', { number: enrollment.current_position + 1 })
  }

  const enrollButton = canEnroll ? (
    <Button size="sm" onClick={() => setEnrolling(true)}>
      <Plus aria-hidden="true" />
      {t('record.enroll')}
    </Button>
  ) : null

  let body
  if (enrollments.isPending) {
    body = (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
      </div>
    )
  } else if (enrollments.isError) {
    body = <ErrorState onRetry={() => void enrollments.refetch()} />
  } else if (enrollments.data.length === 0) {
    body = (
      <EmptyState icon={ListOrdered} title={t('record.emptyTitle')} description={t('record.emptyDescription')} action={enrollButton} />
    )
  } else {
    body = (
      <ul className="grid gap-2" aria-label={t('record.listLabel')}>
        {enrollments.data.map((enrollment) => (
          <li key={enrollment.id} className="flex flex-wrap items-center gap-3 rounded-lg border p-3">
            <div className="grid min-w-0 flex-1 gap-0.5">
              <div className="flex flex-wrap items-center gap-2">
                <Link to={sequencePath(enrollment.sequence_id)} className="truncate font-medium hover:underline">
                  {enrollment.sequence_name}
                </Link>
                <EnrollmentStatusBadge status={enrollment.status} />
              </div>
              <p className="text-xs text-muted-foreground">{detail(enrollment)}</p>
              {enrollment.enrolled_by_name && (
                <p className="text-xs text-muted-foreground">{t('record.enrolledBy', { name: enrollment.enrolled_by_name })}</p>
              )}
            </div>
            {canEnroll && <EnrollmentActions enrollment={enrollment} label={enrollment.sequence_name} />}
          </li>
        ))}
      </ul>
    )
  }

  return (
    <div className="grid gap-3">
      {enrollments.isSuccess && enrollments.data.length > 0 && enrollButton && (
        <div className="flex justify-end">{enrollButton}</div>
      )}
      {body}
      {enrolling && (
        <EnrollDialog
          contactId={contactId}
          enrolledSequenceIds={live.map((item) => item.sequence_id)}
          onClose={() => setEnrolling(false)}
        />
      )}
    </div>
  )
}
