import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast-store'
import { useCancelBooking } from '../api'
import type { MeetingBookingResponse } from '../types'
import { Field } from '@/components/ui/field'

interface CancelBookingDialogProps {
  booking: MeetingBookingResponse | null
  onClose: () => void
}

export function CancelBookingDialog({ booking, onClose }: CancelBookingDialogProps) {
  const { t } = useTranslation('scheduling')
  const cancel = useCancelBooking()
  const [reason, setReason] = useState('')

  return (
    <ConfirmDialog
      open={booking !== null}
      onOpenChange={(open) => {
        if (open) return
        setReason('')
        onClose()
      }}
      variant="destructive"
      notifyOnError={false}
      title={t('bookings.cancelTitle')}
      description={t('bookings.cancelDescription', { name: booking?.guest_name ?? '' })}
      confirmLabel={t('bookings.cancelConfirm')}
      cancelLabel={t('actions.back')}
      onConfirm={async () => {
        if (!booking) return
        try {
          await cancel.mutateAsync({ bookingId: booking.id, reason: reason.trim() || null })
          notifySuccess(t('bookings.canceled'))
        } catch (error) {
          if (isApiError(error) && error.code === 'booking_not_changeable') {
            toast({ title: t('bookings.notChangeable'), variant: 'destructive' })
          } else {
            notifyError(error)
          }
        }
      }}
    >
      <Field label={t('bookings.reason')}>
        {(control) => <Textarea placeholder={t('placeholders.reason')} {...control} rows={3} value={reason} onChange={(event) => setReason(event.target.value)} />}
      </Field>
    </ConfirmDialog>
  )
}
