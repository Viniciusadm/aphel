import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { GripVertical, Settings2, X } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDate, useRecordFormatting } from '@/features/records/formatting'
import { cn } from '@/lib/utils'
import { useWidgetData } from './api'
import type { DashboardSubject, LayoutPeriod, LayoutWidget, ResolvedPeriod, WidgetSettings, WidgetSize } from './types'
import { WidgetSettingsForm } from './WidgetSettingsForm'
import { widgetRenderer } from './widgets/registry'
import { NativeSelect } from '@/components/ui/native-select'

const SIZE_CLASSES: Record<WidgetSize, string> = {
  small: 'md:col-span-1 xl:col-span-1',
  medium: 'md:col-span-2 xl:col-span-2',
  large: 'md:col-span-2 xl:col-span-4',
}

const SIZES: WidgetSize[] = ['small', 'medium', 'large']

export interface WidgetCardProps {
  widget: LayoutWidget
  title: string
  usesPeriod: boolean
  period: LayoutPeriod
  subject: DashboardSubject | null
  compare: boolean
  editing: boolean
  onRemove: () => void
  onResize: (size: WidgetSize) => void
  onSettingsChange: (settings: WidgetSettings) => void
}

function WidgetSkeleton() {
  return (
    <div className="grid gap-2" aria-busy="true">
      <Skeleton className="h-6 w-1/3" />
      <Skeleton className="h-16 w-full" />
      <Skeleton className="h-16 w-full" />
    </div>
  )
}

function PreviousPeriodLabel({ period }: { period: ResolvedPeriod }) {
  const { t } = useTranslation('dashboard')
  const formatting = useRecordFormatting()
  return (
    <p className="mt-2 truncate text-xs text-muted-foreground">
      {t('compare.against', {
        range: t('period.range', { from: formatDate(period.from, formatting), to: formatDate(period.to, formatting) }),
      })}
    </p>
  )
}

function hasPrevious(data: unknown): boolean {
  return typeof data === 'object' && data !== null && 'previous' in data && (data as { previous: unknown }).previous !== null
}

function WidgetBody({
  widget,
  period,
  usesPeriod,
  subject,
  compare,
}: Pick<WidgetCardProps, 'widget' | 'period' | 'usesPeriod' | 'subject' | 'compare'>) {
  const { t } = useTranslation('dashboard')
  const renderer = widgetRenderer(widget.key)
  const query = useWidgetData(
    widget.key,
    { period: usesPeriod ? period : null, subject, compare: usesPeriod && compare },
    widget.settings ?? {},
  )
  if (!renderer) return <EmptyState title={t('states.unsupported')} className="p-4" />
  if (query.isPending) return <WidgetSkeleton />
  if (query.isError) {
    return <ErrorState className="p-4" description={t('states.widgetError')} onRetry={() => void query.refetch()} />
  }
  if (renderer.isEmpty?.(query.data.data)) {
    return <EmptyState title={t('states.widgetEmpty')} className="p-4" />
  }
  return (
    <div className={cn('min-w-0 transition-opacity', query.isFetching && 'opacity-60')}>
      {renderer.render(query.data.data)}
      {query.data.previous_period && hasPrevious(query.data.data) && (
        <PreviousPeriodLabel period={query.data.previous_period} />
      )}
    </div>
  )
}

export function WidgetCard({
  widget,
  title,
  usesPeriod,
  period,
  subject,
  compare,
  editing,
  onRemove,
  onResize,
  onSettingsChange,
}: WidgetCardProps) {
  const { t } = useTranslation('dashboard')
  const formatting = useRecordFormatting()
  const [settingsOpen, setSettingsOpen] = useState(false)
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({
    id: widget.id,
    disabled: !editing,
  })
  const settingsFields = widgetRenderer(widget.key)?.settings
  const headingId = `widget-${widget.id}-title`
  const periodLabel =
    usesPeriod && period.preset === 'custom' && period.from && period.to
      ? t('period.range', { from: formatDate(period.from, formatting), to: formatDate(period.to, formatting) })
      : usesPeriod
        ? t(`period.presets.${period.preset}`)
        : null

  return (
    <Card
      ref={setNodeRef}
      role="region"
      aria-labelledby={headingId}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      className={cn('min-w-0 gap-4 py-4', SIZE_CLASSES[widget.size], isDragging && 'z-10 opacity-80 shadow-lg')}
    >
      <CardHeader className="flex items-start gap-2 px-4">
        {editing && (
          <Button
            ref={setActivatorNodeRef}
            variant="ghost"
            size="icon"
            className="size-8 shrink-0 cursor-grab touch-none"
            aria-label={t('actions.move', { title })}
            {...attributes}
            {...listeners}
          >
            <GripVertical className="size-4" aria-hidden />
          </Button>
        )}
        <div className="grid min-w-0 flex-1 gap-1">
          <CardTitle id={headingId} className="truncate text-base">
            {title}
          </CardTitle>
          {periodLabel && <CardDescription className="truncate text-xs">{periodLabel}</CardDescription>}
        </div>
        {editing && (
          <div className="flex shrink-0 items-center gap-1">
            <NativeSelect
              aria-label={t('actions.size', { title })}
              value={widget.size}
              onChange={(event) => onResize(event.target.value as WidgetSize)}
            >
              {SIZES.map((size) => (
                <option key={size} value={size}>
                  {t(`sizes.${size}`)}
                </option>
              ))}
            </NativeSelect>
            {settingsFields && (
              <Popover open={settingsOpen} onOpenChange={setSettingsOpen}>
                <PopoverTrigger asChild>
                  <Button variant="ghost" size="icon" className="size-8" aria-label={t('actions.settings', { title })}>
                    <Settings2 className="size-4" aria-hidden />
                  </Button>
                </PopoverTrigger>
                <PopoverContent align="end" className="w-64">
                  <WidgetSettingsForm
                    fields={settingsFields}
                    settings={widget.settings ?? {}}
                    onCancel={() => setSettingsOpen(false)}
                    onSubmit={(settings) => {
                      setSettingsOpen(false)
                      onSettingsChange(settings)
                    }}
                  />
                </PopoverContent>
              </Popover>
            )}
            <Button variant="ghost" size="icon" className="size-8" aria-label={t('actions.remove', { title })} onClick={onRemove}>
              <X className="size-4" aria-hidden />
            </Button>
          </div>
        )}
      </CardHeader>
      <CardContent className="min-w-0 px-4">
        <WidgetBody widget={widget} period={period} usesPeriod={usesPeriod} subject={subject} compare={compare} />
      </CardContent>
    </Card>
  )
}
