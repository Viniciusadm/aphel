import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useQueryClient } from '@tanstack/react-query'
import { LoaderCircle, MapPin, MapPinOff, RefreshCw } from 'lucide-react'
import { useCallback, useEffect, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback/notify'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { activitiesKeys, useActivityTypes } from '../api'
import { recordsKeys } from '@/features/records/api'

import { CHECK_IN_TYPE_KEYS, pickActivityType, submitActivity } from './api'
import { captureLocation, mapUrl, type CapturedLocation, type LocationFailure } from '@/features/mobile/lib/geolocation'
import { DictationField } from '@/features/mobile/components/DictationField'
import { RecordPicker, type PickedRecord } from '@/features/mobile/components/RecordPicker'
import { useOutboxFeedback } from '@/features/mobile/lib/outbox-feedback'
import { FormActions, FormBody, type QuickFormProps } from '@/features/mobile/components/QuickForm'
import { Field } from '@/components/ui/field'

type LocationState =
  | { status: 'idle' }
  | { status: 'capturing' }
  | { status: 'ok'; location: CapturedLocation }
  | { status: 'failed'; reason: LocationFailure }

export interface CheckInQuickFormProps extends QuickFormProps {
  initialRecord: PickedRecord | null
}

function LocationStatus({ state, onRetry, onSkip }: { state: LocationState; onRetry: () => void; onSkip: () => void }) {
  const { t } = useTranslation('activities', { keyPrefix: 'quickAdd' })
  if (state.status === 'capturing') {
    return (
      <p className="flex items-center gap-2 text-sm text-muted-foreground" aria-live="polite">
        <LoaderCircle className="size-4 animate-spin" aria-hidden="true" />
        {t('checkIn.capturing')}
      </p>
    )
  }
  if (state.status === 'ok') {
    const { location } = state
    return (
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm" aria-live="polite">
        <span className="flex items-center gap-2">
          <MapPin className="size-4 text-success" aria-hidden="true" />
          {location.accuracy_m !== null
            ? t('checkIn.captured', { accuracy: location.accuracy_m })
            : t('checkIn.capturedNoAccuracy')}
        </span>
        <a
          href={mapUrl(location.latitude, location.longitude)}
          target="_blank"
          rel="noreferrer"
          className="text-primary underline-offset-4 hover:underline"
        >
          {t('checkIn.viewOnMap')}
        </a>
        <Button type="button" variant="ghost" size="sm" onClick={onSkip}>
          {t('checkIn.removeLocation')}
        </Button>
      </div>
    )
  }
  return (
    <div className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground" aria-live="polite">
      <MapPinOff className="size-4" aria-hidden="true" />
      <span className="min-w-0 flex-1">
        {state.status === 'failed' ? t(`checkIn.failure.${state.reason}`) : t('checkIn.withoutLocation')}
      </span>
      <Button type="button" variant="outline" size="sm" className="h-9" onClick={onRetry}>
        <RefreshCw aria-hidden="true" />
        {t(state.status === 'idle' ? 'checkIn.capture' : 'checkIn.retry')}
      </Button>
    </div>
  )
}

export function CheckInQuickForm({ onDone, initialRecord }: CheckInQuickFormProps) {
  const { t } = useTranslation('activities', { keyPrefix: 'quickAdd' })
  const queryClient = useQueryClient()
  const feedback = useOutboxFeedback()
  const types = useActivityTypes()
  const visitType = pickActivityType(types.data, CHECK_IN_TYPE_KEYS)
  const ids = { title: useId(), notes: useId() }
  const [record, setRecord] = useState<PickedRecord | null>(initialRecord)
  const [title, setTitle] = useState('')
  const [notes, setNotes] = useState('')
  const [location, setLocation] = useState<LocationState>({ status: 'capturing' })
  const [pending, setPending] = useState(false)

  const capture = useCallback(async () => {
    setLocation({ status: 'capturing' })
    const result = await captureLocation()
    setLocation(result.status === 'ok' ? { status: 'ok', location: result.location } : result)
  }, [])

  useEffect(() => {
    let cancelled = false
    void captureLocation().then((result) => {
      if (!cancelled) setLocation(result.status === 'ok' ? { status: 'ok', location: result.location } : result)
    })
    return () => {
      cancelled = true
    }
  }, [])

  const resolvedTitle =
    title.trim() ||
    (record ? t('checkIn.defaultTitleWithRecord', { title: record.title }) : t('checkIn.defaultTitle'))

  const submit = async () => {
    if (!visitType) return
    setPending(true)
    try {
      const result = await submitActivity(
        {
          title: resolvedTitle.slice(0, 255),
          activity_type_id: visitType.id,
          status: 'done',
          description: notes.trim() || null,
          record_type: record?.recordType ?? null,
          record_id: record?.recordId ?? null,
          location: location.status === 'ok' ? location.location : null,
        },
        resolvedTitle,
      )
      feedback(result, t('checkIn.created'))
      void queryClient.invalidateQueries({ queryKey: activitiesKeys.all })
      if (record) void queryClient.invalidateQueries({ queryKey: recordsKeys.detail(record.recordType, record.recordId) })
      onDone()
    } catch (error) {
      notifyError(error)
    } finally {
      setPending(false)
    }
  }

  return (
    <FormBody onSubmit={() => void submit()}>
      <div className="grid gap-2 rounded-md border bg-muted/30 p-3">
        <span className="text-sm font-medium">{t('checkIn.location')}</span>
        <LocationStatus
          state={location}
          onRetry={() => void capture()}
          onSkip={() => setLocation({ status: 'idle' })}
        />
      </div>
      <RecordPicker value={record} onChange={setRecord} />
      <Field label={t('checkIn.title')}>
        {(control) => <Input
          {...control}
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          placeholder={resolvedTitle}
          className="h-11 text-base"
          maxLength={255}
        />}
      </Field>
      <DictationField id={ids.notes} label={t('checkIn.notes')} value={notes} onChange={setNotes} maxLength={20_000} />
      {!visitType && !types.isPending && <SharedFieldError>{t('checkIn.noType')}</SharedFieldError>}
      <FormActions
        pending={pending}
        disabled={!visitType || location.status === 'capturing'}
        label={t('checkIn.submit')}
      />
    </FormBody>
  )
}
