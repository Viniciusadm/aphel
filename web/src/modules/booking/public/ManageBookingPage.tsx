import { PageTitle } from '@/components/ui/page-title'
import { CalendarCheck2, CalendarX2, Clock, MapPin, User } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useLocation, useParams } from 'react-router'
import { isApiError } from '@/api'
import { ConfirmDialog, PageLoader } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Textarea } from '@/components/ui/textarea'
import { browserTimeZone } from '@/modules/booking/lib/format'
import { bookingApi, useCancelPublicBooking, usePublicBooking, useReschedule } from './api'
import { BookingLayout } from './components/BookingLayout'
import { InvalidLink } from './components/InvalidLink'
import { SlotPicker } from './components/SlotPicker'
import { formatWhen, timeZoneLabel } from './lib/time'
import type { PublicBookingResponse, PublicSlot } from './types'
import { Alert } from '@/components/ui/alert'
import { Field } from '@/components/ui/field'

interface ManageLocationState {
  booked?: boolean
  timeZone?: string
}

type ManageErrorKey = 'errors.slotUnavailable' | 'errors.notChangeable' | 'errors.rateLimited' | 'errors.generic'

function manageErrorKey(error: unknown): ManageErrorKey {
  if (!isApiError(error)) return 'errors.generic'
  if (error.code === 'booking_slot_unavailable') return 'errors.slotUnavailable'
  if (error.code === 'booking_not_changeable') return 'errors.notChangeable'
  if (error.status === 429) return 'errors.rateLimited'
  return 'errors.generic'
}

interface BookingSummaryProps {
  booking: PublicBookingResponse
  timeZone: string
}

function BookingSummary({ booking, timeZone }: BookingSummaryProps) {
  const { t } = useTranslation('booking', { keyPrefix: 'public' })
  const canceled = booking.status === 'canceled'
  return (
    <div className="grid gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <p className="text-sm font-medium text-muted-foreground">{booking.organization_name}</p>
        <Badge variant={canceled ? 'secondary' : 'success'}>{t(`status.${booking.status}`)}</Badge>
      </div>
      <PageTitle>{booking.meeting_type_name}</PageTitle>
      <ul className="grid gap-2 text-sm">
        <li className="flex items-start gap-2">
          <Clock className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden />
          <span className={canceled ? 'line-through first-letter:uppercase' : 'first-letter:uppercase'}>
            {formatWhen(booking.starts_at, booking.ends_at, timeZone)}
          </span>
        </li>
        {booking.host_name && (
          <li className="flex items-center gap-2">
            <User className="size-4 shrink-0 text-muted-foreground" aria-hidden />
            {booking.host_name}
          </li>
        )}
        {booking.location_value && !canceled && (
          <li className="flex items-start gap-2">
            <MapPin className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden />
            <span className="grid gap-0.5">
              <span className="text-xs text-muted-foreground">{t(`locationKinds.${booking.location_kind}`)}</span>
              {booking.location_kind === 'video' && /^https?:\/\//i.test(booking.location_value) ? (
                <a href={booking.location_value} target="_blank" rel="noreferrer" className="break-all underline">
                  {booking.location_value}
                </a>
              ) : (
                <span className="break-words">{booking.location_value}</span>
              )}
            </span>
          </li>
        )}
      </ul>
      <p className="text-xs text-muted-foreground" title={timeZone}>
        {t('timezone.current', { timezone: timeZoneLabel(timeZone) })}
      </p>
      {canceled && booking.cancel_reason && (
        <p className="text-sm text-muted-foreground">{t('manage.cancelReason', { reason: booking.cancel_reason })}</p>
      )}
    </div>
  )
}

function CancelBookingButton({ token }: { token: string }) {
  const { t } = useTranslation('booking', { keyPrefix: 'public' })
  const cancel = useCancelPublicBooking(token)
  const [open, setOpen] = useState(false)
  const [reason, setReason] = useState('')
  return (
    <>
      <Button variant="outline" onClick={() => setOpen(true)}>
        {t('manage.cancel')}
      </Button>
      <ConfirmDialog
        open={open}
        onOpenChange={setOpen}
        variant="destructive"
        title={t('manage.cancelTitle')}
        description={t('manage.cancelDescription')}
        confirmLabel={t('manage.cancelConfirm')}
        cancelLabel={t('actions.back')}
        onConfirm={async () => {
          await cancel.mutateAsync(reason.trim() || null)
        }}
      >
        <Field label={t('manage.reason')}>
          {(control) => <Textarea placeholder={t('placeholders.reason')} {...control} rows={3} value={reason} onChange={(event) => setReason(event.target.value)} />}
        </Field>
      </ConfirmDialog>
    </>
  )
}

interface RescheduleProps {
  token: string
  timeZone: string
  onTimeZoneChange: (timeZone: string) => void
  onDone: () => void
}

function Reschedule({ token, timeZone, onTimeZoneChange, onDone }: RescheduleProps) {
  const { t } = useTranslation('booking', { keyPrefix: 'public' })
  const reschedule = useReschedule(token)
  const [slot, setSlot] = useState<PublicSlot | null>(null)
  const [errorKey, setErrorKey] = useState<ManageErrorKey | null>(null)

  const confirm = async () => {
    if (!slot) return
    setErrorKey(null)
    try {
      await reschedule.mutateAsync(slot.starts_at)
      onDone()
    } catch (error) {
      setErrorKey(manageErrorKey(error))
      setSlot(null)
    }
  }

  return (
    <div className="grid gap-4">
      <h2 className="text-lg font-semibold">{t('manage.rescheduleTitle')}</h2>
      {errorKey && (
        <Alert>
          {t(errorKey)}
        </Alert>
      )}
      <SlotPicker
        owner={`manage:${token}`}
        fetchSlots={(range) => bookingApi.manageSlots(token, range)}
        timeZone={timeZone}
        onTimeZoneChange={onTimeZoneChange}
        selected={slot}
        onSelect={setSlot}
      />
      <div className="flex flex-wrap justify-end gap-2">
        <Button variant="ghost" onClick={onDone} disabled={reschedule.isPending}>
          {t('actions.back')}
        </Button>
        <Button onClick={() => void confirm()} disabled={!slot || reschedule.isPending}>
          {reschedule.isPending ? t('manage.rescheduling') : t('manage.rescheduleConfirm')}
        </Button>
      </div>
    </div>
  )
}

function ManageBooking({ token, booking }: { token: string; booking: PublicBookingResponse }) {
  const { t } = useTranslation('booking', { keyPrefix: 'public' })
  const location = useLocation()
  const state = (location.state ?? {}) as ManageLocationState
  const [timeZone, setTimeZone] = useState(() => state.timeZone ?? browserTimeZone())
  const [rescheduling, setRescheduling] = useState(false)
  const canceled = booking.status === 'canceled'

  return (
    <div className="grid gap-6 p-5 sm:p-6 md:p-8">
      {state.booked && !canceled && !rescheduling && (
        <div className="flex items-start gap-3 rounded-lg border border-success-solid/30 bg-success-solid/10 p-4">
          <CalendarCheck2 className="size-5 shrink-0" aria-hidden />
          <div className="grid gap-1">
            <p className="font-medium">{t('manage.confirmedTitle')}</p>
            <p className="text-sm text-muted-foreground">
              {t('manage.confirmedDescription', { email: booking.guest_email })}
            </p>
          </div>
        </div>
      )}
      {canceled && (
        <div className="flex items-start gap-3 rounded-lg border bg-muted/50 p-4">
          <CalendarX2 className="size-5 shrink-0" aria-hidden />
          <div className="grid gap-1">
            <p className="font-medium">{t('manage.canceledTitle')}</p>
            <Link to={`/book/${encodeURIComponent(booking.slug)}`} className="text-sm underline">
              {t('manage.bookAgain')}
            </Link>
          </div>
        </div>
      )}
      <BookingSummary booking={booking} timeZone={timeZone} />
      {rescheduling ? (
        <Reschedule
          token={token}
          timeZone={timeZone}
          onTimeZoneChange={setTimeZone}
          onDone={() => setRescheduling(false)}
        />
      ) : (
        booking.can_change &&
        !canceled && (
          <div className="flex flex-wrap gap-2">
            <Button className="flex-1 sm:flex-none" onClick={() => setRescheduling(true)}>
              {t('manage.reschedule')}
            </Button>
            <CancelBookingButton token={token} />
          </div>
        )
      )}
      {!booking.can_change && !canceled && (
        <p className="text-sm text-muted-foreground">{t('manage.locked')}</p>
      )}
    </div>
  )
}

export function ManageBookingPage() {
  const { token = '' } = useParams()
  const booking = usePublicBooking(token)

  if (booking.isPending) {
    return (
      <BookingLayout>
        <PageLoader />
      </BookingLayout>
    )
  }
  if (booking.isError) {
    if (isApiError(booking.error) && booking.error.status === 404) return <InvalidLink />
    return (
      <BookingLayout>
        <div className="p-6">
          <ErrorState onRetry={() => void booking.refetch()} />
        </div>
      </BookingLayout>
    )
  }
  return (
    <BookingLayout>
      <div className="mx-auto w-full max-w-2xl">
        <ManageBooking token={token} booking={booking.data} />
      </div>
    </BookingLayout>
  )
}
