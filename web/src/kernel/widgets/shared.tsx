import { ArrowDownRight, ArrowRight, ArrowUpRight } from 'lucide-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useRecordFormatting } from '@/features/records/formatting'
import { getRecordResource } from '@/features/records/resources'
import { cn } from '@/lib/utils'
import type { WidgetRecordRef } from './types'

export interface StatTileProps {
  label: string
  value: ReactNode
  hint?: ReactNode
  delta?: ReactNode
  className?: string
}

export function StatTile({ label, value, hint, delta, className }: StatTileProps) {
  return (
    <div className={cn('flex min-w-0 flex-col gap-1 rounded-lg border bg-background p-3', className)}>
      <dt className="truncate text-xs font-medium text-muted-foreground">{label}</dt>
      <dd className="truncate text-xl font-semibold tabular-nums">{value}</dd>
      {hint && <dd className="truncate text-xs text-muted-foreground">{hint}</dd>}
      {delta && <dd className="truncate text-xs">{delta}</dd>}
    </div>
  )
}

export interface DeltaProps {
  current: number | null | undefined
  previous: number | null | undefined
  format?: (value: number) => string
  points?: boolean
  inverse?: boolean
}

export function Delta({ current, previous, format, points = false, inverse = false }: DeltaProps) {
  const { t } = useTranslation('common')
  const formatting = useRecordFormatting()
  if (previous === null || previous === undefined || current === null || current === undefined) return null
  const previousText = format ? format(previous) : String(previous)
  const change = points ? current - previous : previous === 0 ? null : (current - previous) / Math.abs(previous)
  if (change === null) {
    return <span className="text-muted-foreground">{t('delta.previousValue', { value: previousText })}</span>
  }
  const Icon = change > 0 ? ArrowUpRight : change < 0 ? ArrowDownRight : ArrowRight
  const better = inverse ? change < 0 : change > 0
  const worse = inverse ? change > 0 : change < 0
  const tone = better ? 'text-success' : worse ? 'text-destructive' : 'text-muted-foreground'
  const amount = points
    ? new Intl.NumberFormat(formatting.locale, { maximumFractionDigits: 1, signDisplay: 'exceptZero' }).format(change * 100)
    : new Intl.NumberFormat(formatting.locale, { style: 'percent', maximumFractionDigits: 1, signDisplay: 'exceptZero' }).format(
        change,
      )
  return (
    <span className="inline-flex max-w-full items-center gap-1">
      <span className={cn('inline-flex items-center gap-0.5 font-medium', tone)}>
        <Icon className="size-3.5 shrink-0" aria-hidden />
        {points ? t('delta.points', { value: amount }) : amount}
      </span>
      <span className="truncate text-muted-foreground">{t('delta.previousValue', { value: previousText })}</span>
    </span>
  )
}

function recordHref(record: WidgetRecordRef): string | null {
  return getRecordResource(record.record_type)?.detailPath?.(record.record_id) ?? null
}

export function RecordLinkText({ record, fallback }: { record: WidgetRecordRef; fallback: string }) {
  const title = record.title ?? fallback
  const href = recordHref(record)
  if (!href) return <span className="font-medium">{title}</span>
  return (
    <Link to={href} className="font-medium underline-offset-2 hover:underline">
      {title}
    </Link>
  )
}
