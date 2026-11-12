import { useEditorialTranslation } from './translation'
import { Dialog, DialogContent, DialogForm, Field, Input } from '../ui'
import { errorMessage } from '@/components/feedback'
import { useEffect, useState } from 'react'

function localInput(date: Date): string {
  const pad = (value: number) => String(value).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`
}

function tomorrowMorning(): string {
  const date = new Date()
  date.setDate(date.getDate() + 1)
  date.setHours(9, 0, 0, 0)
  return localInput(date)
}

interface WhenDialogProps {
  open: boolean
  title: string
  description: string
  submitLabel: string
  onOpenChange: (open: boolean) => void
  onSubmit: (at: string) => Promise<void>
}

export function WhenDialog({ open, title, description, submitLabel, onOpenChange, onSubmit }: WhenDialogProps) {
  const t = useEditorialTranslation()
  const [value, setValue] = useState(tomorrowMorning)
  const [pending, setPending] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!open) return
    setValue(tomorrowMorning())
    setError(null)
  }, [open])

  const chosen = value ? new Date(value) : null
  const past = chosen !== null && chosen.getTime() <= Date.now() + 60_000

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogForm
          title={title}
          description={description}
          error={error}
          pending={pending}
          submitDisabled={!chosen || Number.isNaN(chosen.getTime()) || past}
          submitLabel={submitLabel}
          cancelLabel={t('settings.cancel')}
          onCancel={() => onOpenChange(false)}
          onSubmit={async (event) => {
            event.preventDefault()
            if (!chosen) return
            setPending(true)
            setError(null)
            try {
              await onSubmit(chosen.toISOString())
              onOpenChange(false)
            } catch (failure) {
              setError(errorMessage(failure))
            } finally {
              setPending(false)
            }
          }}
        >
          <Field label={t('schedule.when')} description={t('schedule.whenHelp')} error={past ? t('schedule.past') : undefined}>
            {(control) => <Input {...control} type="datetime-local" value={value} min={localInput(new Date())} onChange={(event) => setValue(event.target.value)} />}
          </Field>
        </DialogForm>
      </DialogContent>
    </Dialog>
  )
}
