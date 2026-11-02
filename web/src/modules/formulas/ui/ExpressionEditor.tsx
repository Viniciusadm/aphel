import { useRef, type ReactNode, type Ref, type UIEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Textarea } from '@/components/ui/textarea'
import type { FormulaError } from '@/modules/formulas/ui/api'
import { FORMULA_EXPRESSION_MAX_LENGTH } from '@/modules/formulas/ui/result-types'

interface ExpressionEditorProps {
  id: string
  value: string
  errors: FormulaError[]
  invalid?: boolean
  describedBy?: string
  textareaRef: Ref<HTMLTextAreaElement>
  name: string
  onChange: (value: string) => void
  onBlur?: () => void
}

interface Range {
  start: number
  end: number
}

function errorRanges(errors: FormulaError[], length: number): Range[] {
  const ranges = errors
    .map((error) => {
      const start = Math.max(0, Math.min(error.start, length))
      const end = Math.max(start + 1, Math.min(Math.max(error.end, error.start), length))
      return { start, end }
    })
    .sort((left, right) => left.start - right.start)
  const merged: Range[] = []
  for (const range of ranges) {
    const last = merged.at(-1)
    if (last && range.start <= last.end) last.end = Math.max(last.end, range.end)
    else merged.push({ ...range })
  }
  return merged
}

function highlighted(value: string, errors: FormulaError[]): ReactNode[] {
  const ranges = errorRanges(errors, value.length)
  const nodes: ReactNode[] = []
  let cursor = 0
  ranges.forEach((range, index) => {
    if (range.start > cursor) nodes.push(value.slice(cursor, range.start))
    const text = value.slice(range.start, range.end)
    nodes.push(
      <mark
        key={index}
        className="rounded-sm bg-destructive/15 text-transparent underline decoration-destructive decoration-wavy underline-offset-4"
      >
        {text.length > 0 ? text : ' '}
      </mark>,
    )
    cursor = Math.max(cursor, range.end)
  })
  if (cursor < value.length) nodes.push(value.slice(cursor))
  nodes.push('\n')
  return nodes
}

export function ExpressionEditor({
  id,
  value,
  errors,
  invalid,
  describedBy,
  textareaRef,
  name,
  onChange,
  onBlur,
}: ExpressionEditorProps) {
  const { t } = useTranslation('formulas')
  const backdropRef = useRef<HTMLDivElement>(null)

  const syncScroll = (event: UIEvent<HTMLTextAreaElement>) => {
    if (!backdropRef.current) return
    backdropRef.current.scrollTop = event.currentTarget.scrollTop
    backdropRef.current.scrollLeft = event.currentTarget.scrollLeft
  }

  return (
    <div className="grid gap-1">
      <div className="relative min-w-0 rounded-md bg-background">
        <div
          ref={backdropRef}
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 overflow-hidden rounded-md border border-transparent px-3 py-2 font-mono text-sm break-words whitespace-pre-wrap text-transparent"
        >
          {highlighted(value, errors)}
        </div>
        <Textarea
          id={id}
          ref={textareaRef}
          name={name}
          value={value}
          rows={4}
          spellCheck={false}
          autoCapitalize="off"
          autoComplete="off"
          autoCorrect="off"
          maxLength={FORMULA_EXPRESSION_MAX_LENGTH}
          placeholder={t('editor.placeholder')}
          aria-invalid={invalid ? true : undefined}
          aria-describedby={describedBy}
          className="relative min-h-28 resize-y bg-transparent font-mono break-words"
          onChange={(event) => onChange(event.target.value)}
          onBlur={onBlur}
          onScroll={syncScroll}
        />
      </div>
      <p className="justify-self-end text-xs text-muted-foreground tabular-nums">
        {t('editor.length', { count: value.length, max: FORMULA_EXPRESSION_MAX_LENGTH })}
      </p>
    </div>
  )
}
