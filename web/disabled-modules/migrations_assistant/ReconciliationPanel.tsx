import { CheckCircle2, CircleAlert } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { formatCurrency, formatDate, formatNumber, useRecordFormatting } from '@/sdk'
import type { MigrationPreview } from './types'

interface Check {
  key: string
  label: string
  value: number
  display: string
  money: boolean
  note?: string
  warn?: boolean
}

export function parseTyped(text: string): number | null {
  const cleaned = text.replace(/[^\d.,-]/g, '')
  if (!/\d/.test(cleaned)) return null
  const lastComma = cleaned.lastIndexOf(',')
  const lastDot = cleaned.lastIndexOf('.')
  const decimalAt = Math.max(lastComma, lastDot)
  const separators = (cleaned.match(/[.,]/g) ?? []).length
  const decimal =
    decimalAt >= 0 && (separators === 1 ? cleaned.length - decimalAt - 1 !== 3 || lastComma > lastDot : true)
  const normalized = decimal
    ? cleaned.slice(0, decimalAt).replace(/[.,]/g, '') + '.' + cleaned.slice(decimalAt + 1)
    : cleaned.replace(/[.,]/g, '')
  const value = Number(normalized)
  return Number.isFinite(value) ? value : null
}

export function ReconciliationPanel({ preview, sourceName }: { preview: MigrationPreview; sourceName: string }) {
  const { t } = useTranslation('migrations_assistant')
  const formatting = useRecordFormatting()
  const [expected, setExpected] = useState<Record<string, string>>({})
  const reconciliation = preview.reconciliation
  const targetLabel = (target: string) => t(`targets.${target as 'name'}`, { defaultValue: target })

  const checks: Check[] = [
    {
      key: 'rows',
      label: t('reconciliation.rows'),
      value: reconciliation.rows - reconciliation.empty_rows,
      display: formatNumber(reconciliation.rows - reconciliation.empty_rows, formatting),
      money: false,
      note: reconciliation.empty_rows > 0 ? t('reconciliation.emptyRows', { count: reconciliation.empty_rows }) : undefined,
    },
    ...reconciliation.money.map((money) => ({
      key: `money.${money.target}`,
      label: t('reconciliation.total', { field: targetLabel(money.target) }),
      value: Number(money.total),
      display: formatCurrency(Number(money.total), formatting),
      money: true,
      note:
        money.filled > money.converted
          ? t('reconciliation.unreadable', { count: money.filled - money.converted })
          : undefined,
      warn: money.filled > money.converted,
    })),
    ...(reconciliation.statuses
      ? (['won', 'lost', 'open'] as const).map((status) => ({
          key: `status.${status}`,
          label: t(`reconciliation.status.${status}`),
          value: reconciliation.statuses?.[status] ?? 0,
          display: formatNumber(reconciliation.statuses?.[status] ?? 0, formatting),
          money: false,
        }))
      : []),
  ]

  return (
    <div className="grid gap-3 rounded-md border bg-muted/30 p-3">
      <div className="grid gap-0.5">
        <h4 className="text-sm font-semibold">{t('reconciliation.title', { source: sourceName })}</h4>
        <p className="text-xs text-muted-foreground">{t('reconciliation.description', { source: sourceName })}</p>
      </div>
      <div className="grid gap-2 text-sm">
        <div className="hidden grid-cols-[minmax(0,1fr)_9rem_10rem_9rem] gap-3 text-xs font-medium text-muted-foreground sm:grid">
          <span>{t('reconciliation.item')}</span>
          <span>{t('reconciliation.file')}</span>
          <span>{t('reconciliation.inSource', { source: sourceName })}</span>
          <span>{t('reconciliation.result')}</span>
        </div>
        {checks.map((check) => {
          const typed = expected[check.key] ?? ''
          const parsed = typed.trim() === '' ? null : parseTyped(typed)
          const matches = parsed !== null && Math.abs(parsed - check.value) < (check.money ? 0.01 : 0.5)
          return (
            <div
              key={check.key}
              className="grid gap-1 sm:grid-cols-[minmax(0,1fr)_9rem_10rem_9rem] sm:items-center sm:gap-3"
            >
              <span className="grid gap-0.5">
                <span>{check.label}</span>
                {check.note && (
                  <span className={check.warn ? 'text-xs text-warning' : 'text-xs text-muted-foreground'}>{check.note}</span>
                )}
              </span>
              <span className="font-medium tabular-nums">{check.display}</span>
              <Input
                value={typed}
                inputMode="decimal"
                placeholder={t('reconciliation.typeHere')}
                aria-label={`${check.label}: ${t('reconciliation.inSource', { source: sourceName })}`}
                onChange={(event) => setExpected((previous) => ({ ...previous, [check.key]: event.target.value }))}
              />
              <span className="min-h-5 text-xs">
                {typed.trim() === '' ? null : parsed === null ? (
                  <span className="text-muted-foreground">{t('reconciliation.notANumber')}</span>
                ) : matches ? (
                  <span className="inline-flex items-center gap-1 text-success">
                    <CheckCircle2 className="size-3.5" aria-hidden="true" />
                    {t('reconciliation.matches')}
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1 text-destructive">
                    <CircleAlert className="size-3.5" aria-hidden="true" />
                    {t('reconciliation.differs', {
                      difference: check.money
                        ? formatCurrency(Math.abs(parsed - check.value), formatting)
                        : formatNumber(Math.abs(parsed - check.value), formatting),
                    })}
                  </span>
                )}
              </span>
            </div>
          )
        })}
      </div>
      {(reconciliation.dates.length > 0 || reconciliation.duplicates > 0) && (
        <ul className="grid gap-1 text-xs text-muted-foreground">
          {reconciliation.dates.map((dateCheck) => (
            <li key={dateCheck.target}>
              {dateCheck.first && dateCheck.last
                ? t('reconciliation.dateRange', {
                    field: targetLabel(dateCheck.target),
                    first: formatDate(dateCheck.first, formatting),
                    last: formatDate(dateCheck.last, formatting),
                    count: dateCheck.converted,
                  })
                : t('reconciliation.noDates', { field: targetLabel(dateCheck.target) })}
              {dateCheck.filled > dateCheck.converted && (
                <span className="text-warning"> · {t('reconciliation.unreadable', { count: dateCheck.filled - dateCheck.converted })}</span>
              )}
            </li>
          ))}
          {reconciliation.duplicates > 0 && (
            <li className="text-warning">{t('reconciliation.duplicates', { count: reconciliation.duplicates })}</li>
          )}
        </ul>
      )}
    </div>
  )
}
