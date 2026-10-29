import { CalendarX2, ChevronLeft, ChevronRight } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Skeleton } from '@/components/ui/skeleton'
import { useBookings, useMeetingTypes } from '../api'
import { formatDateTimeRange } from '../lib/format'
import type { BookingFilters, MeetingBookingResponse } from '../types'
import { CancelBookingDialog } from './CancelBookingDialog'

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
          <Link to={`/contacts/${booking.contact_id}`} className="truncate text-sm font-medium hover:underline">
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
  const lastPage = meta ? Math.max(1, Math.ceil(meta.total / meta.per_page)) : 1

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-end gap-4">
        <div className="grid w-44 gap-1">
          <Label htmlFor="bookings-status">{t('bookings.status')}</Label>
          <Select
            value={filters.status}
            onValueChange={(value) => update({ status: value as BookingFilters['status'] })}
          >
            <SelectTrigger id="bookings-status">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>{t('bookings.allStatuses')}</SelectItem>
              <SelectItem value="confirmed">{t('bookingStatus.confirmed')}</SelectItem>
              <SelectItem value="canceled">{t('bookingStatus.canceled')}</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="grid w-56 gap-1">
          <Label htmlFor="bookings-meeting-type">{t('bookings.meetingType')}</Label>
          <Select
            value={filters.meetingTypeId ?? ALL}
            onValueChange={(value) => update({ meetingTypeId: value === ALL ? null : value })}
          >
            <SelectTrigger id="bookings-meeting-type">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>{t('bookings.allMeetingTypes')}</SelectItem>
              {(meetingTypes.data ?? []).map((meetingType) => (
                <SelectItem key={meetingType.id} value={meetingType.id}>
                  {meetingType.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <label className="flex h-9 items-center gap-2 text-sm">
          <Checkbox
            checked={filters.upcoming}
            onCheckedChange={(checked) => update({ upcoming: checked === true })}
          />
          {t('bookings.upcomingOnly')}
        </label>
      </div>
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
      {lastPage > 1 && (
        <div className="flex items-center justify-end gap-2 text-sm">
          <Button
            variant="outline"
            size="icon"
            aria-label={t('pagination.previous')}
            disabled={filters.page <= 1}
            onClick={() => setFilters((current) => ({ ...current, page: current.page - 1 }))}
          >
            <ChevronLeft />
          </Button>
          <span>{t('pagination.page', { page: filters.page, total: lastPage })}</span>
          <Button
            variant="outline"
            size="icon"
            aria-label={t('pagination.next')}
            disabled={filters.page >= lastPage}
            onClick={() => setFilters((current) => ({ ...current, page: current.page + 1 }))}
          >
            <ChevronRight />
          </Button>
        </div>
      )}
      <CancelBookingDialog booking={canceling} onClose={() => setCanceling(null)} />
    </div>
  )
}
