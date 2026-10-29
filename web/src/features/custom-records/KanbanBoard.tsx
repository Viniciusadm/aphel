import {
  DndContext,
  DragOverlay,
  useDraggable,
  useDroppable,
  type DragEndEvent,
  type DragStartEvent,
} from '@dnd-kit/core'
import { useBoardSensors } from '@/features/mobile/lib/board-sensors'
import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { EllipsisVertical } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { api } from '@/api'
import { notifyError } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Skeleton } from '@/components/ui/skeleton'
import { recordsKeys } from '@/features/records/api'
import { getFieldValue, toRecordPayload } from '@/features/records/fields'
import { recordPath, recordTitle, requireRecordResource, type RecordResource } from '@/features/records/resources'
import { recordListParams } from '@/features/records/table/query'
import type { FieldDefinition, FilterNode, Page, RecordItem } from '@/features/records/types'
import { cn } from '@/lib/utils'

const COLUMN_PAGE_SIZE = 20
const MAX_COLUMN_SIZE = 100
const EMPTY_COLUMN_ID = '__empty__'

interface KanbanColumnDefinition {
  id: string
  value: string | null
  label: string
  color?: string | null
}

export interface KanbanBoardProps {
  recordType: string
  field: FieldDefinition
  search: string
  canEdit: boolean
}

function columnFilter(field: FieldDefinition, value: string | null): FilterNode {
  return value === null
    ? { field: field.key, operator: 'is_empty' }
    : { field: field.key, operator: 'eq', value }
}

function columnIdOf(value: unknown): string {
  return typeof value === 'string' && value ? value : EMPTY_COLUMN_ID
}

export function KanbanBoard({ recordType, field, search, canEdit }: KanbanBoardProps) {
  const { t } = useTranslation('objects')
  const resource = requireRecordResource(recordType)
  const queryClient = useQueryClient()
  const [active, setActive] = useState<RecordItem | null>(null)
  const columns = useMemo<KanbanColumnDefinition[]>(
    () => [
      ...field.options
        .filter((option) => option.is_active)
        .sort((left, right) => left.position - right.position)
        .map((option) => ({ id: option.value, value: option.value, label: option.label, color: option.color })),
      { id: EMPTY_COLUMN_ID, value: null, label: t('kanban.empty') },
    ],
    [field.options, t],
  )

  const move = useMutation({
    mutationFn: ({ record, value }: { record: RecordItem; value: string | null }) =>
      api.patch<RecordItem>(recordPath(resource, record.id), toRecordPayload([field], { [field.key]: value })),
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: recordsKeys.list(recordType) })
    },
    onError: (error) => notifyError(error),
  })

  const moveRecord = (record: RecordItem, value: string | null) => {
    if (columnIdOf(getFieldValue(record, field)) === columnIdOf(value)) return
    move.mutate({ record, value })
  }

  const sensors = useBoardSensors()

  const handleDragStart = (event: DragStartEvent) => {
    setActive((event.active.data.current?.record as RecordItem | undefined) ?? null)
  }

  const handleDragEnd = ({ active: dragged, over }: DragEndEvent) => {
    setActive(null)
    const record = dragged.data.current?.record as RecordItem | undefined
    if (!record || !over) return
    const target = columns.find((column) => column.id === String(over.id))
    if (target) moveRecord(record, target.value)
  }

  return (
    <DndContext
      sensors={sensors}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      onDragCancel={() => setActive(null)}
    >
      <div className="scroll-smooth-thin flex min-h-[24rem] min-w-0 flex-1 snap-x snap-proximity gap-3 overflow-x-auto overscroll-x-contain pb-3 md:snap-none" aria-busy={move.isPending}>
        {columns.map((column) => (
          <KanbanColumn
            key={column.id}
            recordType={recordType}
            resource={resource}
            field={field}
            column={column}
            columns={columns}
            search={search}
            canEdit={canEdit}
            onMove={moveRecord}
          />
        ))}
      </div>
      <DragOverlay dropAnimation={null}>
        {active ? <KanbanCardContent resource={resource} record={active} overlay /> : null}
      </DragOverlay>
    </DndContext>
  )
}

interface KanbanColumnProps {
  recordType: string
  resource: RecordResource
  field: FieldDefinition
  column: KanbanColumnDefinition
  columns: KanbanColumnDefinition[]
  search: string
  canEdit: boolean
  onMove: (record: RecordItem, value: string | null) => void
}

function KanbanColumn({ recordType, resource, field, column, columns, search, canEdit, onMove }: KanbanColumnProps) {
  const { t } = useTranslation('objects')
  const [limit, setLimit] = useState(COLUMN_PAGE_SIZE)
  const { setNodeRef, isOver } = useDroppable({ id: column.id })
  const params = recordListParams({
    page: 1,
    perPage: Math.min(limit, MAX_COLUMN_SIZE),
    filter: columnFilter(field, column.value),
    search,
  })
  const records = useQuery({
    queryKey: [...recordsKeys.list(recordType), 'kanban', params],
    queryFn: ({ signal }) => api.get<Page<RecordItem>>(resource.basePath, { query: params, signal }),
    placeholderData: keepPreviousData,
  })
  const headingId = `kanban-column-${field.key}-${column.id}`
  const items = records.data?.data ?? []
  const total = records.data?.meta.total ?? 0
  const targets = columns.filter((candidate) => candidate.id !== column.id)

  return (
    <section
      aria-labelledby={headingId}
      className={cn(
        'flex w-[min(18rem,85vw)] shrink-0 snap-start flex-col rounded-lg border bg-muted/40 transition-colors md:w-72',
        isOver && 'border-primary bg-primary/5',
      )}
    >
      <header className="flex items-center gap-2 border-b px-3 py-2">
        <span
          className="size-2 shrink-0 rounded-full bg-muted-foreground"
          style={column.color ? { backgroundColor: column.color } : undefined}
          aria-hidden
        />
        <h2 id={headingId} className="min-w-0 flex-1 truncate text-sm font-semibold">
          {column.label}
        </h2>
        <Badge variant="secondary" aria-label={t('kanban.count', { count: total })}>
          {total}
        </Badge>
      </header>
      <ul ref={setNodeRef} className="scroll-smooth-thin flex min-h-24 flex-1 flex-col gap-2 overflow-y-auto p-2">
        {records.isPending ? (
          <>
            <Skeleton className="h-14 w-full" />
            <Skeleton className="h-14 w-full" />
          </>
        ) : records.isError ? (
          <li className="list-none text-xs text-destructive">{t('kanban.loadError')}</li>
        ) : (
          items.map((record) => (
            <KanbanCard
              key={record.id}
              resource={resource}
              record={record}
              targets={targets}
              canEdit={canEdit}
              onMove={onMove}
            />
          ))
        )}
      </ul>
      {items.length < total && limit < MAX_COLUMN_SIZE && (
        <div className="border-t p-2">
          <Button
            variant="ghost"
            size="sm"
            className="w-full"
            disabled={records.isFetching}
            onClick={() => setLimit((current) => current + COLUMN_PAGE_SIZE)}
          >
            {t('kanban.loadMore')}
          </Button>
        </div>
      )}
    </section>
  )
}

interface KanbanCardProps {
  resource: RecordResource
  record: RecordItem
  targets: KanbanColumnDefinition[]
  canEdit: boolean
  onMove: (record: RecordItem, value: string | null) => void
}

function KanbanCard({ resource, record, targets, canEdit, onMove }: KanbanCardProps) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: record.id,
    data: { record },
    disabled: !canEdit,
  })
  const { role: _role, tabIndex: _tabIndex, 'aria-describedby': _describedBy, ...dragAttributes } = attributes

  return (
    <li
      ref={setNodeRef}
      className={cn('list-none touch-manipulation', isDragging && 'opacity-40')}
      {...dragAttributes}
      {...listeners}
    >
      <KanbanCardContent
        resource={resource}
        record={record}
        targets={canEdit ? targets : undefined}
        onMove={canEdit ? onMove : undefined}
      />
    </li>
  )
}

interface KanbanCardContentProps {
  resource: RecordResource
  record: RecordItem
  targets?: KanbanColumnDefinition[]
  onMove?: (record: RecordItem, value: string | null) => void
  overlay?: boolean
}

function KanbanCardContent({ resource, record, targets, onMove, overlay = false }: KanbanCardContentProps) {
  const { t } = useTranslation('objects')
  const title = recordTitle(resource, record)
  const href = resource.detailPath?.(record.id)

  return (
    <div
      className={cn(
        'flex items-start justify-between gap-2 rounded-md border bg-card p-3 text-sm shadow-xs',
        overlay && 'rotate-1 shadow-lg ring-2 ring-primary/40',
      )}
    >
      {href ? (
        <Link
          to={href}
          className="min-w-0 font-medium break-words hover:underline"
          onPointerDown={(event) => event.stopPropagation()}
        >
          {title}
        </Link>
      ) : (
        <span className="min-w-0 font-medium break-words">{title}</span>
      )}
      {onMove && targets && targets.length > 0 && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="-mt-1 -mr-1 size-7 shrink-0"
              aria-label={t('kanban.moveMenu', { title })}
              onPointerDown={(event) => event.stopPropagation()}
            >
              <EllipsisVertical aria-hidden="true" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuLabel>{t('kanban.moveTo')}</DropdownMenuLabel>
            {targets.map((target) => (
              <DropdownMenuItem key={target.id} onSelect={() => onMove(record, target.value)}>
                {target.label}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </div>
  )
}
