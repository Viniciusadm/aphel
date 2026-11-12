import { Globe } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { TimeZoneInput } from '@/modules/booking/lib/TimeZoneInput'
import { isValidTimeZone } from '@/modules/booking/lib/format'
import { timeZoneLabel } from '../lib/time'

interface TimeZonePickerProps {
  value: string
  onChange: (timeZone: string) => void
}

export function TimeZonePicker({ value, onChange }: TimeZonePickerProps) {
  const { t } = useTranslation('booking', { keyPrefix: 'public' })
  const inputId = useId()
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState(value)
  const valid = isValidTimeZone(draft.trim())

  if (!editing) {
    return (
      <div className="flex flex-wrap items-center gap-x-1 text-sm text-muted-foreground">
        <Globe className="size-4" aria-hidden />
        <span title={value}>{t('timezone.current', { timezone: timeZoneLabel(value) })}</span>
        <Button
          variant="link"
          size="sm"
          className="h-auto px-1"
          onClick={() => {
            setDraft(value)
            setEditing(true)
          }}
        >
          {t('timezone.change')}
        </Button>
      </div>
    )
  }

  return (
    <form
      className="flex items-center gap-2"
      onSubmit={(event) => {
        event.preventDefault()
        if (!valid) return
        onChange(draft.trim())
        setEditing(false)
      }}
    >
      <label htmlFor={inputId} className="sr-only">
        {t('timezone.label')}
      </label>
      <TimeZoneInput
        id={inputId}
        className="h-8 w-56"
        value={draft}
        aria-invalid={valid ? undefined : true}
        onChange={(event) => setDraft(event.target.value)}
      />
      <Button type="submit" size="sm" disabled={!valid}>
        {t('timezone.apply')}
      </Button>
      <Button type="button" variant="ghost" size="sm" onClick={() => setEditing(false)}>
        {t('actions.cancel')}
      </Button>
    </form>
  )
}
