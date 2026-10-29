import { Check, Loader2, Pencil, X } from 'lucide-react'
import { useId, useRef, useState, type KeyboardEvent, type MouseEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { CheckboxFieldInput } from '@/components/fields/CheckboxFieldInput'
import { FieldInput } from '@/components/fields/FieldInput'
import { Button } from '@/components/ui/button'
import { FieldValueDisplay } from '../display/FieldValueDisplay'
import { isReadOnlyField } from '../fields'
import type { FieldDefinition } from '../types'
import { fieldErrorMessage } from './lib/errors'
import { issueMessage, validateFieldValue } from './lib/validation'
import { normalizeSubmitValue, valuesEqual } from './lib/values'

export interface InlineFieldEditorProps {
  field: FieldDefinition
  value: unknown
  onSave: (value: unknown) => Promise<void>
  readOnly?: boolean
  knownFields?: FieldDefinition[]
}

function isInteractiveTarget(target: EventTarget) {
  return target instanceof Element && target.closest('a, button, input, textarea, select') !== null
}

export function InlineFieldEditor({ field, value, onSave, readOnly = false, knownFields }: InlineFieldEditorProps) {
  const { t } = useTranslation('records-form')
  const inputId = useId()
  const errorId = `${inputId}-error`
  const wrapperRef = useRef<HTMLDivElement>(null)
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState<unknown>(value)
  const [pending, setPending] = useState<unknown>(undefined)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  if (readOnly || isReadOnlyField(field)) return <FieldValueDisplay field={field} value={value} />

  const save = async (next: unknown): Promise<boolean> => {
    const normalized = normalizeSubmitValue(field, next)
    const issue = validateFieldValue(field, normalized, { initialValue: value })
    if (issue) {
      setError(issueMessage(issue, t))
      return false
    }
    if (valuesEqual(normalized, value)) return true
    setSaving(true)
    setPending(normalized)
    setError(null)
    try {
      await onSave(normalized)
      return true
    } catch (cause) {
      setError(fieldErrorMessage(cause, field.key, t, knownFields))
      return false
    } finally {
      setSaving(false)
      setPending(undefined)
    }
  }

  const errorMessage = error && (
    <p id={errorId} role="alert" className="text-sm text-destructive">
      {error}
    </p>
  )

  if (field.field_type === 'checkbox') {
    const checked = pending === undefined ? value === true : pending === true
    return (
      <div className="grid gap-1">
        <div className="flex min-h-8 items-center gap-2">
          <CheckboxFieldInput
            id={inputId}
            value={checked}
            disabled={saving}
            invalid={Boolean(error)}
            aria-label={field.label}
            aria-describedby={error ? errorId : undefined}
            onChange={(next) => void save(next)}
          />
          {saving && <Loader2 className="size-4 animate-spin text-muted-foreground" aria-label={t('inline.saving')} />}
        </div>
        {errorMessage}
      </div>
    )
  }

  const startEditing = () => {
    setDraft(value)
    setError(null)
    setEditing(true)
  }
  const cancel = () => {
    setEditing(false)
    setError(null)
  }
  const commit = async () => {
    if (await save(draft)) setEditing(false)
  }

  if (!editing) {
    return (
      <div
        className="group -mx-2 flex min-h-8 min-w-0 cursor-pointer items-center gap-1 rounded-md px-2 hover:bg-muted/60"
        onClick={(event: MouseEvent<HTMLDivElement>) => {
          if (!isInteractiveTarget(event.target)) startEditing()
        }}
      >
        <FieldValueDisplay field={field} value={value} className="min-w-0 flex-1 [&>.truncate]:block" />
        <Button
          variant="ghost"
          size="icon"
          className="size-7 shrink-0 opacity-60 group-hover:opacity-100 focus-visible:opacity-100"
          aria-label={t('actions.edit', { field: field.label })}
          onClick={startEditing}
        >
          <Pencil className="size-3.5" />
        </Button>
      </div>
    )
  }

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!(event.target instanceof Node) || !wrapperRef.current?.contains(event.target)) return
    if (event.key === 'Escape') {
      event.preventDefault()
      cancel()
      return
    }
    if (event.key !== 'Enter' || saving) return
    if (event.target instanceof HTMLButtonElement) return
    if (field.field_type === 'long_text' && !event.metaKey && !event.ctrlKey) return
    event.preventDefault()
    void commit()
  }

  return (
    <div ref={wrapperRef} className="grid min-w-0 gap-1" onKeyDown={handleKeyDown}>
      <div className="flex min-w-0 items-start gap-1">
        <div className="min-w-0 flex-1">
          <FieldInput
            field={field}
            id={inputId}
            value={draft}
            onChange={setDraft}
            initialValue={value}
            autoFocus
            disabled={saving}
            invalid={Boolean(error)}
            aria-label={field.label}
            aria-describedby={error ? errorId : undefined}
          />
        </div>
        <Button
          variant="ghost"
          size="icon"
          aria-label={t('inline.save', { field: field.label })}
          disabled={saving}
          onClick={() => void commit()}
        >
          {saving ? <Loader2 className="animate-spin" /> : <Check />}
        </Button>
        <Button variant="ghost" size="icon" aria-label={t('inline.cancel')} disabled={saving} onClick={cancel}>
          <X />
        </Button>
      </div>
      {errorMessage}
    </div>
  )
}
