import { moduleWidgetTitleKey } from '@/kernel/modules/contributions'
import { useModuleState } from '@/kernel/modules/registry'
import { Slot } from '@/kernel/modules/Slot'
import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  TouchSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core'
import { SortableContext, arrayMove, rectSortingStrategy, sortableKeyboardCoordinates } from '@dnd-kit/sortable'
import { LayoutDashboard, Plus, RotateCcw, SlidersHorizontal } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useDashboardLayout, useWidgetCatalog } from './api'
import { PeriodSelector } from './PeriodSelector'
import { CompareToggle, SubjectSelector } from './SubjectSelector'
import type {
  DashboardLayout,
  DashboardSubject,
  LayoutPeriod,
  LayoutWidget,
  WidgetDescriptor,
  WidgetSettings,
  WidgetSize,
} from './types'
import { useLayoutEditor } from './useLayoutEditor'
import { WidgetCard } from './WidgetCard'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

function newWidgetId(): string {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`
  return `w-${random}`
}

function DashboardSkeleton() {
  return (
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4" aria-busy="true">
      <Skeleton className="h-40 md:col-span-2 xl:col-span-4" />
      <Skeleton className="h-64 md:col-span-2" />
      <Skeleton className="h-64 md:col-span-2" />
    </div>
  )
}

export function DashboardPage() {
  const { t } = useTranslation('dashboard')
  const layout = useDashboardLayout()
  const catalog = useWidgetCatalog()
  const { update, reset } = useLayoutEditor()
  const [editing, setEditing] = useState(false)
  const modules = useModuleState()
  const [confirmReset, setConfirmReset] = useState(false)

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(TouchSensor, { activationConstraint: { delay: 200, tolerance: 6 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )

  const descriptors = useMemo(
    () => new Map<string, WidgetDescriptor>((catalog.data ?? []).map((descriptor) => [descriptor.key, descriptor])),
    [catalog.data],
  )

  const current: DashboardLayout | undefined = layout.data
  const widgets = current?.widgets ?? []
  const period = current?.period ?? { preset: 'month' as const }
  const subject = current?.subject ?? null
  const compare = current?.compare_to_previous ?? false

  const titleOf = (key: string) =>
    t((moduleWidgetTitleKey(key, modules) ?? `widgets.${key}.title`) as 'widgets.sales_indicators.title', {
      defaultValue: descriptors.get(key)?.title ?? key,
    })

  const save = (next: {
    widgets?: LayoutWidget[]
    period?: LayoutPeriod
    subject?: DashboardSubject | null
    compare?: boolean
  }) =>
    update({
      widgets: next.widgets ?? widgets,
      period: next.period ?? period,
      subject: next.subject === undefined ? subject : next.subject,
      compare_to_previous: next.compare ?? compare,
    })

  const changeWidget = (id: string, change: Partial<LayoutWidget>) =>
    save({ widgets: widgets.map((widget) => (widget.id === id ? { ...widget, ...change } : widget)) })

  const addWidget = (descriptor: WidgetDescriptor) =>
    save({
      widgets: [
        ...widgets,
        {
          id: newWidgetId(),
          key: descriptor.key,
          size: descriptor.default_size,
          settings: descriptor.default_settings,
        },
      ],
    })

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return
    const from = widgets.findIndex((widget) => widget.id === active.id)
    const to = widgets.findIndex((widget) => widget.id === over.id)
    if (from < 0 || to < 0) return
    save({ widgets: arrayMove(widgets, from, to) })
  }

  const present = new Set(widgets.map((widget) => widget.key))
  const addable = (catalog.data ?? []).filter((descriptor) => !present.has(descriptor.key))

  return (
    <Page>
<PageHeader title={t('title')} description={t('description')} actions={<>
        <div className="flex min-w-0 flex-wrap items-end gap-2">
          {current && <PeriodSelector value={period} onChange={(next) => save({ period: next })} />}
          {current && <SubjectSelector value={subject} onChange={(next) => save({ subject: next })} />}
          {current && <CompareToggle checked={compare} onChange={(next) => save({ compare: next })} />}
          <Button
            variant={editing ? 'default' : 'outline'}
            aria-pressed={editing}
            onClick={() => setEditing((value) => !value)}
            disabled={!current}
          >
            <SlidersHorizontal className="size-4" aria-hidden />
            {editing ? t('actions.done') : t('actions.customize')}
          </Button>
        </div>
      
      </>} />
      <Slot name="dashboard.top" props={{}} />

      {editing && current && (
        <div className="flex flex-wrap items-center gap-2 rounded-lg border border-dashed p-3">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="sm" disabled={catalog.isPending}>
                <Plus className="size-4" aria-hidden />
                {t('actions.addWidget')}
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              {addable.length === 0 ? (
                <DropdownMenuLabel className="font-normal text-muted-foreground">{t('actions.noMoreWidgets')}</DropdownMenuLabel>
              ) : (
                [...new Set(addable.map((descriptor) => descriptor.category))].map((category) => (
                  <DropdownMenuGroup key={category}>
                    {new Set(addable.map((descriptor) => descriptor.category)).size > 1 && (
                      <DropdownMenuLabel className="text-xs text-muted-foreground">
                        {t(`categories.${category}` as never, { defaultValue: category })}
                      </DropdownMenuLabel>
                    )}
                    {addable
                      .filter((descriptor) => descriptor.category === category)
                      .map((descriptor) => (
                        <DropdownMenuItem key={descriptor.key} onSelect={() => addWidget(descriptor)}>
                          {titleOf(descriptor.key)}
                        </DropdownMenuItem>
                      ))}
                  </DropdownMenuGroup>
                ))
              )}
            </DropdownMenuContent>
          </DropdownMenu>
          <Button variant="ghost" size="sm" onClick={() => setConfirmReset(true)} disabled={current.is_default}>
            <RotateCcw className="size-4" aria-hidden />
            {t('actions.reset')}
          </Button>
        </div>
      )}

      {layout.isPending ? (
        <DashboardSkeleton />
      ) : layout.isError ? (
        <ErrorState onRetry={() => void layout.refetch()} />
      ) : widgets.length === 0 ? (
        <EmptyState
          icon={LayoutDashboard}
          title={t('states.emptyLayoutTitle')}
          description={t('states.emptyLayoutDescription')}
          action={
            !editing ? (
              <Button variant="outline" onClick={() => setEditing(true)}>
                {t('actions.customize')}
              </Button>
            ) : undefined
          }
        />
      ) : (
        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
          <SortableContext items={widgets.map((widget) => widget.id)} strategy={rectSortingStrategy}>
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
              {widgets.map((widget) => (
                <WidgetCard
                  key={widget.id}
                  widget={widget}
                  title={titleOf(widget.key)}
                  usesPeriod={descriptors.get(widget.key)?.uses_period ?? true}
                  period={period}
                  subject={subject}
                  compare={compare}
                  editing={editing}
                  onRemove={() => save({ widgets: widgets.filter((item) => item.id !== widget.id) })}
                  onResize={(size: WidgetSize) => changeWidget(widget.id, { size })}
                  onSettingsChange={(settings: WidgetSettings) => changeWidget(widget.id, { settings })}
                />
              ))}
            </div>
          </SortableContext>
        </DndContext>
      )}

      <ConfirmDialog
        open={confirmReset}
        onOpenChange={setConfirmReset}
        title={t('actions.resetTitle')}
        description={t('actions.resetDescription')}
        confirmLabel={t('actions.reset')}
        pending={reset.isPending}
        onConfirm={async () => {
          await reset.mutateAsync()
          setConfirmReset(false)
        }}
      />
    </Page>
  )
}
