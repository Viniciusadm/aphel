import {
  functionalUpdate,
  rowSelectionFeature,
  rowSortingFeature,
  tableFeatures,
  useTable,
  type ColumnDef,
  type Header,
  type RowSelectionState,
  type SortingState,
  type Updater,
} from '@tanstack/react-table'
import { ArrowDown, ArrowUp, ArrowUpDown } from 'lucide-react'
import { useMemo, type MouseEvent, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useLocation, useNavigate } from 'react-router'
import { useMediaQuery } from '../calendar/useMediaQuery'
import { Checkbox } from '@/components/ui/checkbox'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import { FieldValueDisplay } from '../display/FieldValueDisplay'
import { DisplayRecordProvider } from '../display/display-record'
import { getFieldValue } from '../fields'
import { recordTitle, type RecordResource } from '../resources'
import type { FieldDefinition, FieldSchema, RecordItem, SortSpec, TableDensity } from '../types'
import { useFilters } from '@/kernel/modules/filters'
import { useSlotEntries } from '@/kernel/modules/slots'
import { resolveColumnFields } from './columns'
import { MAX_SORTS } from './query'

const features = tableFeatures({ rowSortingFeature, rowSelectionFeature })
type Features = typeof features
type RecordColumn = ColumnDef<Features, RecordItem, unknown>

export const SELECT_COLUMN_ID = '__select'
export const ACTIONS_COLUMN_ID = '__actions'
const EMPTY_IDS: string[] = []
const INTERACTIVE_SELECTOR = 'a, button, input, select, textarea, label, [role="checkbox"], [data-row-click="ignore"]'

export interface RecordTableProps {
  schema: FieldSchema
  records: RecordItem[]
  columns: string[]
  resource?: RecordResource
  sort?: SortSpec[]
  onSortChange?: (sort: SortSpec[]) => void
  mobileSorting?: boolean
  density?: TableDensity
  isLoading?: boolean
  isFetching?: boolean
  loadingRows?: number
  selectedIds?: string[]
  onSelectionChange?: (ids: string[]) => void
  onRowClick?: (record: RecordItem) => void
  rowClassName?: (record: RecordItem) => string | undefined
  titleAddon?: (record: RecordItem) => ReactNode
  caption?: string
  className?: string
}

function toSorting(sort: SortSpec[]): SortingState {
  return sort.map((spec) => ({ id: spec.field, desc: spec.direction === 'desc' }))
}

function fromSorting(sorting: SortingState): SortSpec[] {
  return sorting.slice(0, MAX_SORTS).map((item) => ({ field: item.id, direction: item.desc ? 'desc' : 'asc' }))
}

const cellPadding: Record<TableDensity, string> = {
  compact: 'px-3 py-1.5',
  comfortable: 'px-4 py-3',
}

export function RecordTable({
  schema,
  records,
  columns,
  resource,
  sort = [],
  onSortChange,
  density = 'comfortable',
  mobileSorting = true,
  isLoading = false,
  isFetching = false,
  loadingRows = 8,
  selectedIds = EMPTY_IDS,
  onSelectionChange,
  onRowClick,
  rowClassName,
  titleAddon,
  caption,
  className,
}: RecordTableProps) {
  const { t } = useTranslation('records-table')
  const navigate = useNavigate()
  const location = useLocation()
  const mobile = useMediaQuery('(max-width: 767px)')
  const selectable = Boolean(onSelectionChange)
  const recordType = schema.record_type
  const resolvedFields = useMemo(
    () => resolveColumnFields(schema, columns, resource?.titleField),
    [schema, columns, resource?.titleField],
  )
  const filterContext = useMemo(() => ({ recordType }), [recordType])
  const fields = useFilters('records.columns', resolvedFields, filterContext)
  const rowActions = useSlotEntries('record.list.rowActions', recordType)
  const hasRowActions = rowActions.length > 0
  const titleKey = resource?.titleField ?? fields[0]?.key

  const columnDefs = useMemo<RecordColumn[]>(() => {
    const dataColumns: RecordColumn[] = fields.map((field) => ({
      id: field.key,
      header: field.label,
      accessorFn: (record: RecordItem) => getFieldValue(record, field),
      enableSorting: field.is_sortable && Boolean(onSortChange),
      cell: ({ row }) => (
        <RecordCell
          field={field}
          record={row.original}
          resource={resource}
          isTitle={field.key === titleKey}
          addon={field.key === titleKey ? titleAddon?.(row.original) : undefined}
        />
      ),
    }))
    const withActions: RecordColumn[] = hasRowActions
      ? [...dataColumns, { id: ACTIONS_COLUMN_ID, header: '', enableSorting: false }]
      : dataColumns
    if (!selectable) return withActions
    return [{ id: SELECT_COLUMN_ID, header: '', enableSorting: false }, ...withActions]
  }, [fields, onSortChange, resource, selectable, titleKey, titleAddon, hasRowActions])

  const sorting = useMemo(() => toSorting(sort), [sort])
  const rowSelection = useMemo<RowSelectionState>(
    () => Object.fromEntries(selectedIds.map((id) => [id, true as const])),
    [selectedIds],
  )

  const table = useTable({
    features,
    columns: columnDefs,
    data: records,
    getRowId: (record) => record.id,
    state: { sorting, rowSelection },
    manualSorting: true,
    enableMultiSort: true,
    maxMultiSortColCount: MAX_SORTS,
    sortDescFirst: false,
    enableRowSelection: selectable,
    onSortingChange: (updater: Updater<SortingState>) => onSortChange?.(fromSorting(functionalUpdate(updater, sorting))),
    onRowSelectionChange: (updater: Updater<RowSelectionState>) =>
      onSelectionChange?.(Object.keys(functionalUpdate(updater, rowSelection))),
  })

  const openRecord = (record: RecordItem) => {
    if (onRowClick) onRowClick(record)
    else if (resource?.detailPath) void navigate(resource.detailPath(record.id), { state: { returnTo: `${location.pathname}${location.search}` } })
  }
  const clickable = Boolean(onRowClick || resource?.detailPath)

  const handleRowClick = (event: MouseEvent<HTMLTableRowElement>, record: RecordItem) => {
    if (!clickable) return
    if ((event.target as HTMLElement).closest(INTERACTIVE_SELECTOR)) return
    if (window.getSelection()?.toString()) return
    openRecord(record)
  }

  const padding = cellPadding[density]
  const headerGroups = table.getHeaderGroups()
  const rows = table.getRowModel().rows

  if (mobile) return <div className={cn('grid min-w-0 gap-3', className)} aria-busy={isLoading || isFetching}>
    {mobileSorting && onSortChange && <details className="rounded-lg border p-3">
      <summary className="min-h-8 cursor-pointer text-sm font-medium">{t('table.sortOptions')}</summary>
      <div className="grid gap-2 pt-3">{fields.filter((field) => field.is_sortable).map((field) => {
        const current = sort.find((item) => item.field === field.key)
        return <button key={field.key} type="button" className="flex min-h-11 items-center justify-between gap-2 rounded-md px-2 text-left text-sm hover:bg-muted" onClick={() => {
          const others = sort.filter((item) => item.field !== field.key)
          onSortChange(current?.direction === 'desc' ? others : [{ field: field.key, direction: current ? 'desc' as const : 'asc' as const }, ...others].slice(0, MAX_SORTS))
        }}>{field.label}{current && <span>{current.direction === 'asc' ? '↑' : '↓'} {sort.findIndex((item) => item.field === field.key) + 1}</span>}</button>
      })}</div>
    </details>}
    {selectable && <label className="flex min-h-11 items-center gap-3 text-sm"><Checkbox checked={table.getIsAllPageRowsSelected() ? true : table.getIsSomePageRowsSelected() ? 'indeterminate' : false} disabled={isLoading || !rows.length} onCheckedChange={(value) => table.toggleAllPageRowsSelected(value === true)} />{t('table.selectPage')}</label>}
    {isLoading ? Array.from({ length: 3 }, (_, index) => <Skeleton key={index} className="h-40 w-full" />) : rows.map((row) => <DisplayRecordProvider key={row.id} value={{ recordType, recordId: row.original.id, title: resource ? recordTitle(resource, row.original) : row.original.id }}><article className={cn('grid min-w-0 gap-2 border-b bg-card px-3 py-3', row.getIsSelected() && 'border-primary bg-accent/40', rowClassName?.(row.original))}>
      <div className="flex min-w-0 items-start gap-3">
        {selectable && <Checkbox aria-label={t('table.selectRow', { title: resource ? recordTitle(resource, row.original) : row.original.id })} checked={row.getIsSelected()} onCheckedChange={(value) => row.toggleSelected(value === true)} />}
        <div className="min-w-0 flex-1">{resource?.detailPath ? <Link state={{ returnTo: `${location.pathname}${location.search}` }} to={resource.detailPath(row.original.id)} className="block min-h-6 text-sm font-semibold break-words">{recordTitle(resource, row.original)}</Link> : <button type="button" className="text-left text-sm font-semibold" onClick={() => openRecord(row.original)}>{resource ? recordTitle(resource, row.original) : row.original.id}</button>}{titleAddon?.(row.original)}</div>
      </div>
      <dl className="grid grid-cols-2 gap-x-4 gap-y-2">{fields.filter((field) => field.key !== titleKey).slice(0, 3).map((field) => <div key={field.key} className="grid gap-1"><dt className="text-xs text-muted-foreground">{field.label}</dt><dd className="min-w-0 text-sm break-words"><FieldValueDisplay field={field} value={getFieldValue(row.original, field)} /></dd></div>)}</dl>
      {fields.filter((field) => field.key !== titleKey).length > 3 && <details><summary className="min-h-11 cursor-pointer text-sm font-medium text-primary">{t('table.moreFields')}</summary><dl className="grid gap-3 pt-2">{fields.filter((field) => field.key !== titleKey).slice(3).map((field) => <div key={field.key} className="grid gap-1"><dt className="text-xs text-muted-foreground">{field.label}</dt><dd className="min-w-0 text-sm break-words"><FieldValueDisplay field={field} value={getFieldValue(row.original, field)} /></dd></div>)}</dl></details>}
      {hasRowActions && <div className="flex flex-wrap gap-2 border-t pt-3">{rowActions.map(({ key, Component }) => <Component key={key} recordType={recordType} record={row.original} />)}</div>}
    </article></DisplayRecordProvider>)}
  </div>

  return (
    <div
      className={cn('relative min-w-0 overflow-x-auto rounded-lg border', className)}
      data-density={density}
      aria-busy={isLoading || isFetching}
    >
      <table className="w-full text-sm">
        <caption className="sr-only">{caption ?? t('table.caption')}</caption>
        <thead className="bg-muted/50 text-left text-muted-foreground">
          {headerGroups.map((group) => (
            <tr key={group.id}>
              {group.headers.map((header) =>
                header.column.id === SELECT_COLUMN_ID ? (
                  <th key={header.id} scope="col" className={cn('w-10', padding)}>
                    <Checkbox
                      aria-label={t('table.selectPage')}
                      disabled={isLoading || rows.length === 0}
                      checked={
                        table.getIsAllPageRowsSelected() ? true : table.getIsSomePageRowsSelected() ? 'indeterminate' : false
                      }
                      onCheckedChange={(checked) => table.toggleAllPageRowsSelected(checked === true)}
                    />
                  </th>
                ) : (
                  <SortableHeader key={header.id} header={header} padding={padding} multiSorted={sort.length > 1} />
                ),
              )}
            </tr>
          ))}
        </thead>
        <tbody className={cn(isFetching && !isLoading && 'opacity-60 transition-opacity')}>
          {isLoading
            ? Array.from({ length: loadingRows }, (_, index) => (
                <tr key={`loading-${index}`} className="border-t" data-testid="record-table-skeleton">
                  {columnDefs.map((column) => (
                    <td key={column.id} className={padding}>
                      <Skeleton className={cn('h-4', column.id === SELECT_COLUMN_ID ? 'w-4' : 'w-24')} />
                    </td>
                  ))}
                </tr>
              ))
            : rows.map((row) => (
                <tr
                  key={row.id}
                  data-state={row.getIsSelected() ? 'selected' : undefined}
                  className={cn(
                    'border-t transition-colors data-[state=selected]:bg-accent/60',
                    clickable && 'cursor-pointer hover:bg-muted/40',
                    rowClassName?.(row.original),
                  )}
                  onClick={(event) => handleRowClick(event, row.original)}
                >
                  {row.getAllCells().map((cell) =>
                    cell.column.id === SELECT_COLUMN_ID ? (
                      <td key={cell.id} className={cn('w-10', padding)}>
                        <Checkbox
                          aria-label={t('table.selectRow', {
                            title: resource ? recordTitle(resource, row.original) : row.original.id,
                          })}
                          checked={row.getIsSelected()}
                          onCheckedChange={(checked) => row.toggleSelected(checked === true)}
                        />
                      </td>
                    ) : cell.column.id === ACTIONS_COLUMN_ID ? (
                      <td key={cell.id} className={cn('w-px align-middle whitespace-nowrap', padding)}>
                        <div className="flex items-center justify-end gap-1">
                          {rowActions.map(({ key, Component }) => (
                            <Component key={key} recordType={recordType} record={row.original} />
                          ))}
                        </div>
                      </td>
                    ) : (
                      <td key={cell.id} className={cn('max-w-80 align-middle', padding)}>
                        <table.FlexRender cell={cell} />
                      </td>
                    ),
                  )}
                </tr>
              ))}
        </tbody>
      </table>
      {isLoading && <span className="sr-only">{t('table.loading')}</span>}
    </div>
  )
}

interface SortableHeaderProps {
  header: Header<Features, RecordItem, unknown>
  padding: string
  multiSorted: boolean
}

function SortableHeader({ header, padding, multiSorted }: SortableHeaderProps) {
  const { t } = useTranslation('records-table')
  const column = header.column
  const label = typeof column.columnDef.header === 'string' ? column.columnDef.header : column.id
  const direction = column.getIsSorted()
  const sortIndex = column.getSortIndex()
  const ariaSort = direction === 'asc' ? 'ascending' : direction === 'desc' ? 'descending' : undefined

  if (!column.getCanSort()) {
    return (
      <th scope="col" className={cn('font-medium whitespace-nowrap', padding)}>
        {label}
      </th>
    )
  }

  const Icon = direction === 'asc' ? ArrowUp : direction === 'desc' ? ArrowDown : ArrowUpDown
  return (
    <th scope="col" aria-sort={ariaSort} className={cn('font-medium whitespace-nowrap', padding)}>
      <button
        type="button"
        className="-mx-1 inline-flex items-center gap-1 rounded-sm px-1 outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
        title={t('table.sortHint', { max: MAX_SORTS })}
        aria-label={t('table.sortBy', { label })}
        onClick={column.getToggleSortingHandler()}
      >
        <span>{label}</span>
        <Icon className={cn('size-3.5', !direction && 'opacity-40')} aria-hidden />
        {direction && multiSorted && sortIndex >= 0 && (
          <span className="text-[0.625rem] tabular-nums" aria-hidden>
            {sortIndex + 1}
          </span>
        )}
        {direction && (
          <span className="sr-only">{direction === 'asc' ? t('table.sortAscending') : t('table.sortDescending')}</span>
        )}
      </button>
    </th>
  )
}

interface RecordCellProps {
  field: FieldDefinition
  record: RecordItem
  resource?: RecordResource
  isTitle: boolean
  addon?: ReactNode
}

function RecordCell({ field, record, resource, isTitle, addon }: RecordCellProps): ReactNode {
  const value = getFieldValue(record, field)
  const location = useLocation()
  if (isTitle && resource) {
    const title = recordTitle(resource, record)
    const href = resource.detailPath?.(record.id)
    const label = href ? (
      <Link to={href} state={{ returnTo: `${location.pathname}${location.search}` }} className="block truncate font-medium text-foreground underline-offset-4 hover:underline">
        {title}
      </Link>
    ) : (
      <span className="block truncate font-medium">{title}</span>
    )
    if (!addon) return label
    return (
      <div className="grid min-w-0 gap-1">
        {label}
        {addon}
      </div>
    )
  }
  const display = <FieldValueDisplay field={field} value={value} className="block truncate" />
  if (field.field_type !== 'phone' || !resource) return display
  return (
    <DisplayRecordProvider value={{ recordType: resource.recordType, recordId: record.id, title: recordTitle(resource, record) }}>
      {display}
    </DisplayRecordProvider>
  )
}
