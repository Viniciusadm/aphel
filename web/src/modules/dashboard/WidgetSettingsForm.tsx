import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import type {WidgetSettings} from '@/sdk'
import type { SettingsField } from '@/kernel/widgets/types'
import { Field } from '@/components/ui/field'
import { ToggleField } from '@/components/ui/toggle-field'

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
          <Field key={field.key} id={`${id}-${field.key}`} label={t(field.label as 'title')}>
            <Input placeholder={t('placeholders.number')}
              id={`${id}-${field.key}`}
              type="number"
              inputMode="numeric"
              min={field.min}
              max={field.max}
              value={String(draft[field.key] ?? '')}
              onChange={(event) => setDraft((current) => ({ ...current, [field.key]: event.target.value }))}
            />
          </Field>
        ) : (
          <ToggleField
            key={field.key}
            label={t(field.label as 'title')}
            checked={draft[field.key] === true}
            onCheckedChange={(checked) => setDraft((current) => ({ ...current, [field.key]: checked }))}
          />
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
