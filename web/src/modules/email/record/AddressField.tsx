import { X } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Label } from '@/components/ui/label'
import { cn } from '@/lib/utils'
import { isValidAddress, normalizeAddress } from '../lib/format'
import type { EmailAddress } from '../types'

interface AddressFieldProps {
  label: string
  value: EmailAddress[]
  onChange: (value: EmailAddress[]) => void
  suggestions: EmailAddress[]
  autoFocus?: boolean
}

const SEPARATORS = /[,;\s]+/

function display(entry: EmailAddress): string {
  return entry.name ? `${entry.name} <${entry.address}>` : entry.address
}

export function AddressField({ label, value, onChange, suggestions, autoFocus }: AddressFieldProps) {
  const { t } = useTranslation('email')
  const id = useId()
  const [draft, setDraft] = useState('')
  const [invalid, setInvalid] = useState<string | null>(null)
  const selected = new Set(value.map((entry) => normalizeAddress(entry.address)))
  const available = suggestions.filter((entry) => !selected.has(normalizeAddress(entry.address)))

  const commit = (raw: string) => {
    const tokens = raw.split(SEPARATORS).map((token) => token.trim()).filter(Boolean)
    if (tokens.length === 0) return
    const next = [...value]
    const rejected: string[] = []
    for (const token of tokens) {
      const address = token.replace(/^<|>$/g, '')
      if (!isValidAddress(address)) {
        rejected.push(token)
        continue
      }
      if (next.some((entry) => normalizeAddress(entry.address) === normalizeAddress(address))) continue
      const known = suggestions.find((entry) => normalizeAddress(entry.address) === normalizeAddress(address))
      next.push(known ?? { address })
    }
    onChange(next)
    setDraft(rejected.join(', '))
    setInvalid(rejected.length > 0 ? rejected.join(', ') : null)
  }

  return (
    <div className="grid gap-2.5">
      <Label htmlFor={id}>{label}</Label>
      <div
        className={cn(
          'flex min-h-9 flex-wrap items-center gap-1 rounded-md border border-input bg-background px-2 py-1 shadow-xs focus-within:ring-2 focus-within:ring-ring',
          invalid && 'border-destructive-solid',
        )}
      >
        {value.map((entry) => (
          <span
            key={entry.address}
            className="inline-flex max-w-full items-center gap-1 rounded-md bg-secondary px-2 py-0.5 text-xs text-secondary-foreground"
            title={display(entry)}
          >
            <span className="truncate">{entry.name || entry.address}</span>
            <button
              type="button"
              className="rounded-sm opacity-70 hover:opacity-100 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
              aria-label={t('composer.removeRecipient', { address: entry.address })}
              onClick={() => onChange(value.filter((item) => item.address !== entry.address))}
            >
              <X className="size-3" />
            </button>
          </span>
        ))}
        <input
          id={id}
          type="email"
          multiple
          list={`${id}-suggestions`}
          value={draft}
          autoFocus={autoFocus}
          autoComplete="off"
          autoCapitalize="off"
          spellCheck={false}
          aria-invalid={invalid ? true : undefined}
          aria-describedby={invalid ? `${id}-error` : undefined}
          className="h-7 min-w-32 flex-1 bg-transparent text-sm outline-none [&::-webkit-calendar-picker-indicator]:hidden"
          onChange={(event) => {
            const next = event.target.value
            if (SEPARATORS.test(next.slice(-1))) commit(next)
            else {
              setDraft(next)
              if (suggestions.some((entry) => entry.address === next.trim())) commit(next)
            }
          }}
          onKeyDown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault()
              commit(draft)
            }
            if (event.key === 'Backspace' && draft === '' && value.length > 0) onChange(value.slice(0, -1))
          }}
          onBlur={() => commit(draft)}
          onPaste={(event) => {
            const text = event.clipboardData.getData('text')
            if (SEPARATORS.test(text)) {
              event.preventDefault()
              commit(`${draft}${text}`)
            }
          }}
        />
        <datalist id={`${id}-suggestions`}>
          {available.map((entry) => (
            <option key={entry.address} value={entry.address}>
              {entry.name ?? entry.address}
            </option>
          ))}
        </datalist>
      </div>
      {invalid && (
        <p id={`${id}-error`} className="text-sm text-destructive">
          {t('composer.invalidAddress', { value: invalid })}
        </p>
      )}
    </div>
  )
}
