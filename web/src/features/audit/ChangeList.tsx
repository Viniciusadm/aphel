import { ArrowRight } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import type { AuditFieldChange } from './api'
import { displayValue } from './lib/values'

function ChangeValue({ value }: { value: unknown }) {
  const { t } = useTranslation('audit')
  const display = displayValue(value)
  if (display.kind === 'redacted') {
    return (
      <Badge variant="secondary" title={t('changes.redactedHint')}>
        {t('changes.redacted')}
      </Badge>
    )
  }
  if (display.kind === 'empty') {
    return <span className="text-muted-foreground italic">{t('changes.empty')}</span>
  }
  return <span className="break-all">{display.text}</span>
}

export function ChangeList({ changes }: { changes: AuditFieldChange[] }) {
  const { t } = useTranslation('audit')
  if (changes.length === 0) return <span className="text-muted-foreground">{t('changes.none')}</span>
  return (
    <ul className="grid gap-1" aria-label={t('changes.label')}>
      {changes.map((change) => (
        <li key={change.field} className="flex flex-wrap items-center gap-x-1.5 gap-y-0.5">
          <span className="font-medium">{change.field}:</span>
          <ChangeValue value={change.old} />
          <ArrowRight className="size-3.5 text-muted-foreground" aria-label={t('changes.to')} />
          <ChangeValue value={change.new} />
        </li>
      ))}
    </ul>
  )
}
