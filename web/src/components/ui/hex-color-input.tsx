import { useState } from 'react'
import { Input } from '@/components/ui/input'

const HEX = /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i

function expand(hex: string) {
  const value = hex.toLowerCase()
  return value.length === 4 ? `#${value[1]}${value[1]}${value[2]}${value[2]}${value[3]}${value[3]}` : value
}

export function HexColorInput({ value, label, disabled, fallback = '#000000', placeholder, onChange }: { value: string; label: string; disabled?: boolean; fallback?: string; placeholder?: string; onChange: (value: string) => void }) {
  const [draft, setDraft] = useState<string | null>(null)
  const text = draft ?? value
  const commit = (next: string) => {
    const candidate = next.trim().startsWith('#') ? next.trim() : `#${next.trim()}`
    if (HEX.test(candidate)) onChange(expand(candidate))
  }
  return (
    <span className="flex min-w-0 items-center gap-1.5">
      <input type="color" className="size-8 shrink-0 cursor-pointer rounded-md border border-border bg-transparent p-0.5 disabled:cursor-not-allowed" aria-label={label} value={HEX.test(value) ? expand(value) : fallback} disabled={disabled} onChange={(event) => onChange(event.target.value)} />
      <Input
        aria-label={label}
        className="h-8 min-w-0 px-2 font-mono text-xs"
        value={text}
        maxLength={7}
        placeholder={placeholder}
        spellCheck={false}
        autoComplete="off"
        disabled={disabled}
        aria-invalid={text !== '' && !HEX.test(text.startsWith('#') ? text : `#${text}`) ? true : undefined}
        onChange={(event) => {
          setDraft(event.target.value)
          commit(event.target.value)
        }}
        onBlur={() => setDraft(null)}
      />
    </span>
  )
}
