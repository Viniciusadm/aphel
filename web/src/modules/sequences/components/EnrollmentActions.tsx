import { LogOut, Pause, Play } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { useEnrollmentAction, type EnrollmentAction } from '../api'
import { notifySequenceError } from '../lib/errors'
import type { Enrollment } from '../types'

export interface EnrollmentActionsProps {
  enrollment: Enrollment
  label: string
}

export function EnrollmentActions({ enrollment, label }: EnrollmentActionsProps) {
  const { t } = useTranslation('sequences')
  const action = useEnrollmentAction()
  const [confirmingExit, setConfirmingExit] = useState(false)
  const live = enrollment.status === 'active' || enrollment.status === 'paused'
  if (!live) return null

  const run = async (kind: EnrollmentAction) => {
    try {
      await action.mutateAsync({ enrollmentId: enrollment.id, action: kind })
      notifySuccess(t(`enrollments.toast.${kind}`))
    } catch (error) {
      notifySequenceError(t, error, t('enrollments.toast.failed'))
    } finally {
      setConfirmingExit(false)
    }
  }

  return (
    <div className="flex justify-end gap-1">
      {enrollment.status === 'active' ? (
        <Button
          variant="ghost"
          size="icon"
          aria-label={`${t('enrollments.pause')}: ${label}`}
          disabled={action.isPending}
          onClick={() => void run('pause')}
        >
          <Pause />
        </Button>
      ) : (
        <Button
          variant="ghost"
          size="icon"
          aria-label={`${t('enrollments.resume')}: ${label}`}
          disabled={action.isPending}
          onClick={() => void run('resume')}
        >
          <Play />
        </Button>
      )}
      <Button
        variant="ghost"
        size="icon"
        aria-label={`${t('enrollments.exit')}: ${label}`}
        disabled={action.isPending}
        onClick={() => setConfirmingExit(true)}
      >
        <LogOut />
      </Button>
      {confirmingExit && (
        <ConfirmDialog
          open
          variant="destructive"
          title={t('enrollments.confirmExit.title', { name: label })}
          description={t('enrollments.confirmExit.description')}
          confirmLabel={t('enrollments.exit')}
          pending={action.isPending}
          onConfirm={() => run('exit')}
          onOpenChange={(open) => !open && setConfirmingExit(false)}
        />
      )}
    </div>
  )
}
