import { PaginationSummary } from '@/components/ui/pagination-summary'
import { NativeSelect } from '@/components/ui/native-select'
import { CalendarX2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useBookings, useMeetingTypes } from '../api'
import { formatDateTimeRange } from '../lib/format'
import type { BookingFilters, MeetingBookingResponse } from '../types'
import { CancelBookingDialog } from './CancelBookingDialog'
import { Field } from '@/components/ui/field'
import { Pagination } from '@/components/ui/pagination'

const ALL = 'all'

interface BookingRowProps {
  booking: MeetingBookingResponse
  now: number
  onCancel: () => void
}

function BookingRow({ booking, now, onCancel }: BookingRowProps) {
  const { t } = useTranslation('scheduling')
  const canceled = booking.status === 'canceled'
  const upcoming = new Date(booking.ends_at).getTime() > now
  return (
    <li className="grid gap-2 p-4 md:grid-cols-[minmax(0,14rem)_minmax(0,1fr)_minmax(0,1fr)_auto] md:items-center">
      <div className="grid gap-0.5">
        <span className="text-sm font-medium">{formatDateTimeRange(booking.starts_at, booking.ends_at)}</span>
        <Badge variant={canceled ? 'secondary' : 'success'}>{t(`bookingStatus.${booking.status}`)}</Badge>
      </div>
      <div className="grid min-w-0 gap-0.5">
        {booking.contact_id ? (
          <Link to={`/people/${booking.contact_id}`} className="truncate text-sm font-medium hover:underline">
            {booking.guest_name}
          </Link>
        ) : (
          <span className="truncate text-sm font-medium">{booking.guest_name}</span>
        )}
        <span className="truncate text-xs text-muted-foreground">{booking.guest_email}</span>
      </div>
      <div className="grid min-w-0 gap-0.5 text-sm">
        <span className="truncate">{booking.meeting_type.name}</span>
        <span className="truncate text-xs text-muted-foreground">
          {t('bookings.host', { name: booking.host.name })}
        </span>
        {canceled && booking.cancel_reason && (
          <span className="truncate text-xs text-muted-foreground">
            {t('bookings.cancelReason', { reason: booking.cancel_reason })}
          </span>
        )}
      </div>
      <div className="flex justify-end">
        {!canceled && upcoming && (
          <Button variant="outline" size="sm" onClick={onCancel}>
            {t('bookings.cancel')}
          </Button>
        )}
      </div>
    </li>
  )
}

export function BookingsPanel() {
  const { t } = useTranslation('scheduling')
  const [filters, setFilters] = useState<BookingFilters>({
    status: 'confirmed',
    meetingTypeId: null,
    upcoming: true,
    page: 1,
  })
  const [canceling, setCanceling] = useState<MeetingBookingResponse | null>(null)
  const [now] = useState(() => Date.now())
  const bookings = useBookings(filters)
  const meetingTypes = useMeetingTypes()
  const update = (patch: Partial<BookingFilters>) => setFilters((current) => ({ ...current, page: 1, ...patch }))
  const meta = bookings.data?.meta

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-end gap-4">
        <Field label={t('bookings.status')} className="w-44">
          {(fieldControl) => <NativeSelect value={filters.status}
            onValueChange={(value) => update({ status: value as BookingFilters['status'] })} id={fieldControl.id}>
              <option value={ALL}>{t('bookings.allStatuses')}</option>
              <option value="confirmed">{t('bookingStatus.confirmed')}</option>
              <option value="canceled">{t('bookingStatus.canceled')}</option>
            </NativeSelect>}
        </Field>
        <Field label={t('bookings.meetingType')} className="w-56">
          {(fieldControl) => <NativeSelect value={filters.meetingTypeId ?? ALL}
            onValueChange={(value) => update({ meetingTypeId: value === ALL ? null : value })} id={fieldControl.id}>
              <option value={ALL}>{t('bookings.allMeetingTypes')}</option>
              {(meetingTypes.data ?? []).map((meetingType) => (
                <option key={meetingType.id} value={meetingType.id}>
                  {meetingType.name}
                </option>
              ))}
            </NativeSelect>}
        </Field>
        <label className="flex h-9 items-center gap-2 text-sm">
          <Checkbox
            checked={filters.upcoming}
            onCheckedChange={(checked) => update({ upcoming: checked === true })}
          />
          {t('bookings.upcomingOnly')}
        </label>
      </div>
      {meta && <PaginationSummary page={filters.page} perPage={meta.per_page} total={meta.total} />}
      {bookings.isPending ? (
        <div className="grid gap-2">
          <Skeleton className="h-16" />
          <Skeleton className="h-16" />
        </div>
      ) : bookings.isError ? (
        <ErrorState onRetry={() => void bookings.refetch()} />
      ) : bookings.data.data.length === 0 ? (
        <EmptyState icon={CalendarX2} title={t('bookings.emptyTitle')} description={t('bookings.emptyDescription')} />
      ) : (
        <ul className="divide-y rounded-lg border">
          {bookings.data.data.map((booking) => (
            <BookingRow key={booking.id} booking={booking} now={now} onCancel={() => setCanceling(booking)} />
          ))}
        </ul>
      )}
      {meta && (
        <Pagination
          page={filters.page}
          perPage={meta.per_page}
          total={meta.total}
          onPageChange={(page) => setFilters((current) => ({ ...current, page }))}
        />
      )}
      <CancelBookingDialog booking={canceling} onClose={() => setCanceling(null)} />
    </div>
  )
}
