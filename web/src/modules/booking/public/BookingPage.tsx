import { useQueryClient } from '@tanstack/react-query'
import { ArrowLeft } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useParams } from 'react-router'
import { isApiError } from '@/api'
import { PageLoader } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { browserTimeZone } from '@/modules/booking/lib/format'
import { serverFieldViolations } from '@/modules/booking/lib/errors'
import i18n from '@/i18n'
import { bookingApi, bookingKeys, useBook, usePublicMeetingType } from './api'
import { BookingForm, type BookingFormFieldError, type BookingFormSubmit } from './components/BookingForm'
import { BookingLayout } from './components/BookingLayout'
import { InvalidLink } from './components/InvalidLink'
import { MeetingDetails } from './components/MeetingDetails'
import { SlotPicker } from './components/SlotPicker'
import { formatWhen } from './lib/time'
import type { PublicMeetingTypeResponse, PublicSlot } from './types'
import { Alert } from '@/components/ui/alert'

type BookingErrorKey =
  | 'errors.slotUnavailable'
  | 'errors.rateLimited'
  | 'errors.rejected'
  | 'errors.invalid'
  | 'errors.generic'

function bookingErrorKey(error: unknown): BookingErrorKey {
  if (!isApiError(error)) return 'errors.generic'
  if (error.code === 'booking_slot_unavailable') return 'errors.slotUnavailable'
  if (error.status === 429 || error.code === 'rate_limited') return 'errors.rateLimited'
  if (error.code === 'booking_rejected') return 'errors.rejected'
  if (error.code === 'validation_error') return 'errors.invalid'
  return 'errors.generic'
}

function fieldErrors(error: unknown): BookingFormFieldError[] {
  return serverFieldViolations(error).flatMap((violation): BookingFormFieldError[] => {
    if (violation.field === 'name' || violation.field === 'email') return [violation.field]
    const match = /^answers[.[]([^\]]+)\]?$/.exec(violation.field)
    return match ? [{ question: match[1] }] : []
  })
}

function BookingFlow({ slug, meetingType }: { slug: string; meetingType: PublicMeetingTypeResponse }) {
  const { t } = useTranslation('booking', { keyPrefix: 'public' })
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const book = useBook(slug)
  const [timeZone, setTimeZone] = useState(browserTimeZone)
  const [slot, setSlot] = useState<PublicSlot | null>(null)
  const [step, setStep] = useState<'slot' | 'details'>('slot')
  const [errorKey, setErrorKey] = useState<BookingErrorKey | null>(null)
  const owner = `slug:${slug}`

  const submit = async (values: BookingFormSubmit): Promise<BookingFormFieldError[] | void> => {
    if (!slot) return
    setErrorKey(null)
    try {
      const booking = await book.mutateAsync({
        starts_at: slot.starts_at,
        name: values.name,
        email: values.email,
        timezone: timeZone,
        locale: i18n.resolvedLanguage ?? i18n.language,
        answers: values.answers,
        website: values.website,
      })
      navigate(`/book/manage/${encodeURIComponent(booking.manage_token)}`, {
        state: { booked: true, timeZone },
      })
    } catch (error) {
      const key = bookingErrorKey(error)
      setErrorKey(key)
      if (key === 'errors.slotUnavailable') {
        setSlot(null)
        setStep('slot')
        void queryClient.invalidateQueries({ queryKey: bookingKeys.slots(owner) })
      }
      return fieldErrors(error)
    }
  }

  return (
    <div className="grid gap-6 md:grid-cols-[minmax(0,18rem)_minmax(0,1fr)]">
      <aside className="border-b p-6 md:border-r md:border-b-0">
        <MeetingDetails
          organizationName={meetingType.organization_name}
          meetingName={meetingType.name}
          durationMinutes={meetingType.duration_minutes}
          hostName={meetingType.host_name ?? null}
          locationKind={meetingType.location_kind}
          description={meetingType.description}
        />
      </aside>
      <section className="grid content-start gap-4 p-6 md:pl-0">
        {errorKey && (
          <Alert>
            {t(errorKey)}
          </Alert>
        )}
        <div className={step === 'slot' ? 'grid gap-4' : 'hidden'}>
          <h2 className="text-lg font-semibold">{t('slots.title')}</h2>
          <SlotPicker
            owner={owner}
            fetchSlots={(range) => bookingApi.slots(slug, range)}
            timeZone={timeZone}
            onTimeZoneChange={setTimeZone}
            selected={slot}
            onSelect={(next) => {
              setSlot(next)
              setErrorKey(null)
              setStep('details')
            }}
          />
        </div>
        <div className={step === 'details' && slot ? 'grid gap-4' : 'hidden'}>
          <div className="grid gap-2">
            <Button variant="ghost" size="sm" className="w-fit px-2" onClick={() => setStep('slot')}>
              <ArrowLeft />
              {t('form.changeTime')}
            </Button>
            <h2 className="text-lg font-semibold">{t('form.title')}</h2>
            {slot && (
              <p className="text-sm text-muted-foreground first-letter:uppercase">
                {formatWhen(slot.starts_at, slot.ends_at, timeZone)}
              </p>
            )}
            <p className="text-xs text-muted-foreground">{t('timezone.current', { timezone: timeZone })}</p>
          </div>
          <BookingForm questions={meetingType.questions} submitting={book.isPending} onSubmit={submit} />
        </div>
      </section>
    </div>
  )
}

export function BookingPage() {
  const { slug = '' } = useParams()
  const meetingType = usePublicMeetingType(slug)

  if (meetingType.isPending) {
    return (
      <BookingLayout>
        <PageLoader />
      </BookingLayout>
    )
  }
  if (meetingType.isError) {
    if (isApiError(meetingType.error) && meetingType.error.status === 404) return <InvalidLink />
    return (
      <BookingLayout>
        <div className="p-6">
          <ErrorState onRetry={() => void meetingType.refetch()} />
        </div>
      </BookingLayout>
    )
  }
  return (
    <BookingLayout>
      <BookingFlow slug={slug} meetingType={meetingType.data} />
    </BookingLayout>
  )
}
