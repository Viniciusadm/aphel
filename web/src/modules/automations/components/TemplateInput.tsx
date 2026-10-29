import { Braces } from 'lucide-react'
import { useId, useMemo, useRef, useState, type ChangeEvent, type KeyboardEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Textarea } from '@/components/ui/textarea'
import { cn } from '@/lib/utils'
import { TOKEN_BEFORE_CARET, filterVariables, type TemplateVariable } from '../lib/variables'
import { VariableList } from './VariableList'

const MAX_SUGGESTIONS = 8

export interface TemplateInputProps {
  id?: string
  value: string
  onChange: (value: string) => void
  variables: TemplateVariable[]
  multiline?: boolean
  placeholder?: string
  invalid?: boolean
  maxLength?: number
  'aria-describedby'?: string
}

interface Token {
  start: number
  end: number
  query: string
}

type Field = HTMLInputElement | HTMLTextAreaElement

export function TemplateInput({
  id,
  value,
  onChange,
  variables,
  multiline = false,
  placeholder,
  invalid,
  maxLength,
  'aria-describedby': describedBy,
}: TemplateInputProps) {
  const { t } = useTranslation('automations')
  const inputRef = useRef<Field | null>(null)
  const listId = useId()
  const [token, setToken] = useState<Token | null>(null)
  const [highlight, setHighlight] = useState(0)
  const [pickerOpen, setPickerOpen] = useState(false)

  const suggestions = useMemo(
    () => (token ? filterVariables(variables, token.query).slice(0, MAX_SUGGESTIONS) : []),
    [token, variables],
  )

  const detect = (text: string, caret: number) => {
    const match = TOKEN_BEFORE_CARET.exec(text.slice(0, caret))
    if (!match) {
      setToken(null)
      return
    }
    setToken({ start: match.index, end: caret, query: match[1] ?? '' })
    setHighlight(0)
  }

  const place = (next: string, caret: number) => {
    onChange(next)
    requestAnimationFrame(() => {
      const element = inputRef.current
      if (!element) return
      element.focus()
      element.setSelectionRange(caret, caret)
    })
  }

  const insert = (variable: TemplateVariable) => {
    const snippet = `{{${variable.path}}}`
    if (token) {
      const next = value.slice(0, token.start) + snippet + value.slice(token.end).replace(/^[^{}\s]*\}\}/, '')
      place(next, token.start + snippet.length)
      setToken(null)
      return
    }
    const element = inputRef.current
    const start = element?.selectionStart ?? value.length
    const end = element?.selectionEnd ?? value.length
    place(value.slice(0, start) + snippet + value.slice(end), start + snippet.length)
  }

  const handleChange = (event: ChangeEvent<Field>) => {
    onChange(event.target.value)
    detect(event.target.value, event.target.selectionStart ?? event.target.value.length)
  }

  const handleKeyDown = (event: KeyboardEvent<Field>) => {
    if (!token || suggestions.length === 0) return
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      setHighlight((current) => (current + 1) % suggestions.length)
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      setHighlight((current) => (current - 1 + suggestions.length) % suggestions.length)
    } else if (event.key === 'Enter' || event.key === 'Tab') {
      event.preventDefault()
      const chosen = suggestions[highlight]
      if (chosen) insert(chosen)
    } else if (event.key === 'Escape') {
      event.preventDefault()
      setToken(null)
    }
  }

  const shared = {
    id,
    value,
    placeholder,
    maxLength,
    onChange: handleChange,
    onKeyDown: handleKeyDown,
    onBlur: () => setTimeout(() => setToken(null), 150),
    onClick: (event: { currentTarget: Field }) =>
      detect(event.currentTarget.value, event.currentTarget.selectionStart ?? 0),
    'aria-invalid': invalid ? true : undefined,
    'aria-describedby': describedBy,
    'aria-autocomplete': 'list' as const,
    'aria-controls': token && suggestions.length > 0 ? listId : undefined,
    'aria-expanded': Boolean(token && suggestions.length > 0),
    role: 'combobox',
  }

  return (
    <div className="relative flex min-w-0 items-start gap-1">
      {multiline ? (
        <Textarea
          {...shared}
          ref={(element) => {
            inputRef.current = element
          }}
          rows={4}
          className="min-w-0 flex-1"
        />
      ) : (
        <Input
          {...shared}
          ref={(element) => {
            inputRef.current = element
          }}
          className="min-w-0 flex-1"
        />
      )}
      <Popover open={pickerOpen} onOpenChange={setPickerOpen}>
        <PopoverTrigger asChild>
          <Button type="button" variant="outline" size="icon" aria-label={t('variables.insert')} title={t('variables.insert')}>
            <Braces aria-hidden="true" />
          </Button>
        </PopoverTrigger>
        <PopoverContent align="end" className="w-80 max-w-[calc(100vw-2rem)] p-0">
          <VariableList
            variables={variables}
            onPick={(variable) => {
              setPickerOpen(false)
              insert(variable)
            }}
          />
        </PopoverContent>
      </Popover>
      {token && suggestions.length > 0 && (
        <ul
          id={listId}
          role="listbox"
          aria-label={t('variables.suggestions')}
          className="absolute top-full left-0 z-50 mt-1 max-h-64 w-full max-w-sm overflow-y-auto rounded-md border bg-popover p-1 text-popover-foreground shadow-md"
        >
          {suggestions.map((variable, index) => (
            <li
              key={variable.path}
              role="option"
              aria-selected={index === highlight}
              onMouseDown={(event) => {
                event.preventDefault()
                insert(variable)
              }}
              onMouseEnter={() => setHighlight(index)}
              className={cn(
                'grid cursor-pointer gap-0.5 rounded-sm px-2 py-1.5 text-sm',
                index === highlight && 'bg-accent text-accent-foreground',
              )}
            >
              <span className="truncate">{variable.label}</span>
              <code className="truncate text-xs text-muted-foreground">{`{{${variable.path}}}`}</code>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
