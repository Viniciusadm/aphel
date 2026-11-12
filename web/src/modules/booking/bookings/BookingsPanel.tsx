import { PaginationSummary } from '@/components/ui/pagination-summary'
import { SegmentedControl } from '@/components/ui/segmented-control'
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
import { formatBookingDate, formatBookingTimes } from '../lib/format'
import type { BookingFilters, MeetingBookingResponse } from '../types'
import { CancelBookingDialog } from './CancelBookingDialog'
import { Field } from '@/components/ui/field'
import { Pagination } from '@/components/ui/pagination'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

const ALL = 'all'

interface BookingRowProps {
  booking: MeetingBookingResponse
  now: number
  onCancel: () => void
}

function BookingRow({ booking, now, onCancel }: BookingRowProps) {
  const { t } = useTranslation('booking')
  const canceled = booking.status === 'canceled'
  const upcoming = new Date(booking.ends_at).getTime() > now
  return (
    <TableRow>
      <TableCell className="whitespace-nowrap font-medium">{formatBookingDate(booking.starts_at)}</TableCell>
      <TableCell className="whitespace-nowrap">{formatBookingTimes(booking.starts_at, booking.ends_at)}</TableCell>
      <TableCell>
        <div className="grid min-w-0 gap-0.5">
          {booking.contact_id ? (
            <Link to={`/people/${booking.contact_id}`} className="truncate font-medium hover:underline">
              {booking.guest_name}
            </Link>
          ) : (
            <span className="truncate font-medium">{booking.guest_name}</span>
          )}
          <span className="truncate text-xs text-muted-foreground">{booking.guest_email}</span>
        </div>
      </TableCell>
      <TableCell className="hidden md:table-cell">{booking.meeting_type.name}</TableCell>
      <TableCell className="hidden lg:table-cell">{booking.host.name}</TableCell>
      <TableCell>
        <div className="grid gap-0.5">
          <Badge variant={canceled ? 'secondary' : 'success'} className="w-fit">
            {t(`bookingStatus.${booking.status}`)}
          </Badge>
          {canceled && booking.cancel_reason && (
            <span className="max-w-56 truncate text-xs text-muted-foreground" title={booking.cancel_reason}>
              {t('bookings.cancelReason', { reason: booking.cancel_reason })}
            </span>
          )}
        </div>
      </TableCell>
      <TableCell className="text-right">
        {!canceled && upcoming && (
          <Button variant="outline" size="sm" onClick={onCancel}>
            {t('bookings.cancel')}
          </Button>
        )}
      </TableCell>
    </TableRow>
  )
}

export function BookingsPanel() {
  const { t } = useTranslation('booking')
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
      <div className="flex flex-wrap items-center gap-4">
        <SegmentedControl
          aria-label={t('bookings.status')}
          size="sm"
          value={filters.status}
          onValueChange={(value) => update({ status: value as BookingFilters['status'] })}
          options={[
            { value: 'confirmed', label: t('bookingStatus.confirmed') },
            { value: 'canceled', label: t('bookingStatus.canceled') },
            { value: ALL, label: t('bookings.statusAll') },
          ]}
        />
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
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">{t('bookings.columns.date')}</TableHead>
              <TableHead scope="col">{t('bookings.columns.time')}</TableHead>
              <TableHead scope="col">{t('bookings.columns.guest')}</TableHead>
              <TableHead scope="col" className="hidden md:table-cell">
                {t('bookings.meetingType')}
              </TableHead>
              <TableHead scope="col" className="hidden lg:table-cell">
                {t('bookings.columns.host')}
              </TableHead>
              <TableHead scope="col">{t('bookings.status')}</TableHead>
              <TableHead scope="col">
                <span className="sr-only">{t('bookings.columns.actions')}</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {bookings.data.data.map((booking) => (
              <BookingRow key={booking.id} booking={booking} now={now} onCancel={() => setCanceling(booking)} />
            ))}
          </TableBody>
        </Table>
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
