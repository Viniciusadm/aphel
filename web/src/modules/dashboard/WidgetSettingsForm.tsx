import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import type { WidgetSettings } from './types'
import type { SettingsField } from '@/kernel/widgets/types'

export interface WidgetSettingsFormProps {
  fields: SettingsField[]
  settings: WidgetSettings
  onSubmit: (settings: WidgetSettings) => void
  onCancel: () => void
}

export function WidgetSettingsForm({ fields, settings, onSubmit, onCancel }: WidgetSettingsFormProps) {
  const { t } = useTranslation('dashboard')
  const id = useId()
  const [draft, setDraft] = useState<Record<string, string | boolean>>(() =>
    Object.fromEntries(
      fields.map((field) => [
        field.key,
        field.kind === 'boolean' ? settings[field.key] === true : String(settings[field.key] ?? ''),
      ]),
    ),
  )

  const invalid = fields.some((field) => {
    if (field.kind !== 'integer') return false
    const value = Number(draft[field.key])
    return !Number.isInteger(value) || value < field.min || value > field.max
  })

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (invalid) return
    const next: WidgetSettings = { ...settings }
    for (const field of fields) {
      next[field.key] = field.kind === 'integer' ? Number(draft[field.key]) : draft[field.key] === true
    }
    onSubmit(next)
  }

  return (
    <form className="grid gap-3" onSubmit={submit}>
      {fields.map((field) =>
        field.kind === 'integer' ? (
          <div key={field.key} className="grid gap-1.5">
            <Label htmlFor={`${id}-${field.key}`}>{t(field.label as 'title')}</Label>
            <Input placeholder={t('placeholders.number')}
              id={`${id}-${field.key}`}
              type="number"
              inputMode="numeric"
              min={field.min}
              max={field.max}
              value={String(draft[field.key] ?? '')}
              onChange={(event) => setDraft((current) => ({ ...current, [field.key]: event.target.value }))}
            />
          </div>
        ) : (
          <Label key={field.key} htmlFor={`${id}-${field.key}`} className="font-normal">
            <Checkbox
              id={`${id}-${field.key}`}
              checked={draft[field.key] === true}
              onCheckedChange={(checked) => setDraft((current) => ({ ...current, [field.key]: checked === true }))}
            />
            {t(field.label as 'title')}
          </Label>
        ),
      )}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="ghost" size="sm" onClick={onCancel}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" size="sm" disabled={invalid}>
          {t('actions.save')}
        </Button>
      </div>
    </form>
  )
}
