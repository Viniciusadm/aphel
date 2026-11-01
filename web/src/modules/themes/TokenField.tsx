import { NativeSelect } from '@/components/ui/native-select'
import { RotateCcw } from 'lucide-react'
import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cssColorToHex } from './lib/color'
import { defaultValue, type TokenDefinition } from './lib/tokens'
import type { FontStack, TokenValue } from './lib/types'
import { Field } from '@/components/ui/field'

const FALLBACKS = ['sans-serif', 'serif', 'monospace', 'system-ui', 'ui-sans-serif', 'ui-serif', 'ui-monospace'] as const

interface TokenFieldProps {
  token: TokenDefinition
  mode: 'light' | 'dark'
  value: TokenValue | undefined
  onChange: (value: TokenValue | undefined) => void
}

function label(name: string): string {
  return name.replace(/-/g, ' ')
}

export function TokenField({ token, mode, value, onChange }: TokenFieldProps) {
  const { t } = useTranslation('themes')
  const id = useId()
  const fallback = defaultValue(token, mode)
  return (
    <Field
      id={id}
      label={<span className="capitalize">{label(token.name)}</span>}
      labelAction={
        value !== undefined && (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-6"
            aria-label={t('editor.resetToken', { name: label(token.name) })}
            onClick={() => onChange(undefined)}
          >
            <RotateCcw className="size-3.5" aria-hidden="true" />
          </Button>
        )
      }
    >
      {token.kind === 'font' ? (
        <FontStackField id={id} value={value} placeholder={fallback} onChange={onChange} />
      ) : (
        <div className="flex items-center gap-2">
          {token.kind === 'color' && <ColorSwatch value={typeof value === 'string' ? value : fallback} onPick={onChange} />}
          <Input
            id={id}
            value={typeof value === 'string' ? value : ''}
            placeholder={fallback}
            spellCheck={false}
            autoComplete="off"
            onChange={(event) => onChange(event.target.value.trim() === '' ? undefined : event.target.value)}
          />
        </div>
      )}
    </Field>
  )
}

function ColorSwatch({ value, onPick }: { value: string; onPick: (value: string) => void }) {
  const { t } = useTranslation('themes')
  const hex = cssColorToHex(value, () => undefined)
  return (
    <input
      type="color"
      aria-label={t('editor.pickColor')}
      className="size-9 shrink-0 cursor-pointer rounded-md border bg-transparent p-0.5"
      value={hex ?? '#000000'}
      onChange={(event) => onPick(event.target.value)}
    />
  )
}

function FontStackField({
  id,
  value,
  placeholder,
  onChange,
}: {
  id: string
  value: TokenValue | undefined
  placeholder: string
  onChange: (value: TokenValue | undefined) => void
}) {
  const stack: FontStack | null = typeof value === 'object' ? value : null
  const update = (families: string[], fallbackKind: string) =>
    onChange(families.length === 0 ? undefined : { families, fallback: fallbackKind })
  return (
    <div className="grid gap-2 sm:grid-cols-[minmax(0,1fr)_10rem]">
      <Input
        id={id}
        value={stack?.families.join(', ') ?? ''}
        placeholder={placeholder}
        spellCheck={false}
        onChange={(event) =>
          update(
            event.target.value
              .split(',')
              .map((family) => family.trim())
              .filter(Boolean),
            stack?.fallback ?? 'sans-serif',
          )
        }
      />
      <NativeSelect value={stack?.fallback ?? 'sans-serif'} onValueChange={(next) => update(stack?.families ?? [], next)} aria-label="fallback">
          {FALLBACKS.map((fallback) => (
            <option key={fallback} value={fallback}>
              {fallback}
            </option>
          ))}
        </NativeSelect>
    </div>
  )
}
