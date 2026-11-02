import { RecordListActions } from './RecordListActions'
import { MobileRecordToolbar, type MobileListOptions } from './MobileRecordToolbar'
import { useMediaQuery } from '../calendar/useMediaQuery'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { CalendarDays, SearchX, SlidersHorizontal, Table2 } from 'lucide-react'
import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { isApiError } from '@/api'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useRecordListExtensions } from '@/kernel/modules/record-list'
import { Slot } from '@/kernel/modules/Slot'
import { useSlot } from '@/kernel/modules/slots'
import { cn } from '@/lib/utils'
import { useFieldSchema } from '@/features/records/api'
import { useSavedViews } from '@/modules/saved_views/api'
import {
  CALENDAR_FIELD_SETTING,
  calendarFields,
  pickCalendarField,
  viewCalendarConfig,
  type ListLayout,
} from '../calendar/layout'
import { RecordCalendar } from '../calendar/RecordCalendar'
import { FilterChips } from '../filters/FilterChips'
import { FilterMenu } from '../filters/FilterMenu'
import { NativeSelect } from '../filters/NativeSelect'
import { requireRecordResource } from '../resources'
import type { ArchivedMode, FieldSchema, FilterNode, RecordItem, SavedView, SortSpec, TableDensity } from '../types'
import { useListUrlState } from '@/modules/saved_views/ui/useListUrlState'
import {
  ALL_RECORDS_VIEW,
  hasExplicitQuery,
  pickDefaultView,
  resolveConfig,
  sameConfig,
  viewConfig,
  type ListUrlState,
  buildListSearch,
  parseListSearch,
} from '@/modules/saved_views/ui/urlState'
import { ViewSwitcher } from '@/modules/saved_views/ui/ViewSwitcher'
import { ColumnsMenu } from './ColumnsMenu'
import { defaultColumnKeys } from './columns'
import { DensityToggle } from './DensityToggle'
import { PageSizeSelect } from '@/components/ui/page-size-select'
import { PaginationSummary } from '@/components/ui/pagination-summary'
import { Pagination } from '@/components/ui/pagination'
import { RecordTable } from './RecordTable'
import { SearchBox } from './SearchBox'
import { useRecordList } from './useRecordList'
import { ARCHIVED_PARAM, parseArchivedParam } from './archived'

const EMPTY_RECORDS: RecordItem[] = []
const EMPTY_SELECTION: string[] = []

export interface BulkSelectionQuery {
  filter: FilterNode | null
  search: string
  archived: ArchivedMode
}

export interface BulkActionContext {
  recordType: string
  selectedIds: string[]
  allMatching: boolean
  total: number
  query: BulkSelectionQuery
  clearSelection: () => void
}

export interface RecordListViewProps {
  recordType: string
  emptyAction?: ReactNode
  toolbarActions?: ReactNode
  bulkActions?: (context: BulkActionContext) => ReactNode
  onRowClick?: (record: RecordItem) => void
  rowClassName?: (record: RecordItem) => string | undefined
  titleAddon?: (record: RecordItem) => ReactNode
  baseFilter?: FilterNode
  archived?: ArchivedMode
  archivedFilter?: boolean
  className?: string
}

export function RecordListView(props: RecordListViewProps) {
  const schema = useFieldSchema(props.recordType)
  if (schema.isPending) return <RecordListSkeleton />
  if (schema.isError) return <ErrorState onRetry={() => void schema.refetch()} />
  return <RecordListContent {...props} schema={schema.data} />
}

function RecordListSkeleton() {
  return (
    <div className="grid gap-3" aria-busy="true">
      <div className="flex flex-wrap gap-2">
        <Skeleton className="h-8 w-40" />
        <Skeleton className="h-8 w-56" />
        <Skeleton className="h-8 w-24" />
      </div>
      <Skeleton className="h-64 w-full" />
    </div>
  )
}

interface RecordListContentProps extends RecordListViewProps {
  schema: FieldSchema
}

function RecordListContent({
  recordType,
  schema,
  emptyAction,
  toolbarActions,
  bulkActions,
  onRowClick,
  rowClassName,
  titleAddon,
  archived,
  archivedFilter = false,
  className,
  baseFilter,
}: RecordListContentProps) {
  const { t } = useTranslation('records-table')
  const mobile = useMediaQuery('(max-width: 767px)')
  const resource = requireRecordResource(recordType)
  const viewsQuery = useSavedViews(recordType)
  const [urlState, setUrlState] = useListUrlState()
  const [searchParams, setSearchParams] = useSearchParams()
  const archivedMode: ArchivedMode =
    archived ?? (archivedFilter ? parseArchivedParam(searchParams.get(ARCHIVED_PARAM)) : 'exclude')
  const views = useMemo(() => viewsQuery.data ?? [], [viewsQuery.data])
  const defaultView = pickDefaultView(views)
  const needsDefault =
    viewsQuery.isSuccess && urlState.view === null && !hasExplicitQuery(urlState) && Boolean(defaultView)

  useEffect(() => {
    if (needsDefault && defaultView) {
      setUrlState((current) => ({ ...current, view: defaultView.id }), { replace: true })
    }
  }, [needsDefault, defaultView, setUrlState])

  const requestedView = urlState.view && urlState.view !== ALL_RECORDS_VIEW ? urlState.view : null
  const activeView = requestedView ? views.find((view) => view.id === requestedView) : undefined
  const viewMissing = Boolean(requestedView) && viewsQuery.isSuccess && !activeView

  const defaultColumns = useMemo(() => defaultColumnKeys(schema, resource.titleField), [schema, resource.titleField])
  const baseConfig = useMemo(() => viewConfig(activeView, defaultColumns), [activeView, defaultColumns])
  const config = useMemo(() => resolveConfig(urlState, baseConfig), [urlState, baseConfig])
  const dateFields = useMemo(() => calendarFields(schema), [schema])
  const viewCalendar = viewCalendarConfig(activeView)
  const baseLayout: ListLayout = dateFields.length > 0 ? viewCalendar.layout : 'table'
  const baseCalendarField = pickCalendarField(dateFields, viewCalendar.field)
  const layout: ListLayout = dateFields.length > 0 ? (urlState.layout ?? baseLayout) : 'table'
  const calendarField = pickCalendarField(dateFields, urlState.calendarField ?? viewCalendar.field)
  const showCalendar = layout === 'calendar' && calendarField !== null
  const layoutModified =
    layout !== baseLayout || (layout === 'calendar' && calendarField?.key !== baseCalendarField?.key)
  const isModified = !sameConfig(config, baseConfig) || layoutModified
  const listFilter = useMemo<FilterNode | null>(() => {
    if (!baseFilter) return config.filter
    if (!config.filter) return baseFilter
    return { op: 'and', conditions: [baseFilter, config.filter] }
  }, [baseFilter, config.filter])
  const ready = !viewsQuery.isPending && !needsDefault

  const list = useRecordList(
    recordType,
    {
      page: urlState.page,
      perPage: urlState.perPage,
      filter: listFilter,
      sort: config.sort,
      search: urlState.search,
      archived: archivedMode === 'exclude' ? archived : archivedMode,
    },
    { enabled: ready && !showCalendar },
  )

  const selectionScope = JSON.stringify([requestedView, listFilter, urlState.search, archivedMode])
  const [selection, setSelection] = useState<{ scope: string; ids: string[]; allMatching: boolean }>({
    scope: selectionScope,
    ids: [],
    allMatching: false,
  })
  const currentSelection = selection.scope === selectionScope ? selection : null
  const selectedIds = currentSelection?.ids ?? EMPTY_SELECTION
  const allMatching = currentSelection?.allMatching ?? false
  const setSelectedIds = (ids: string[]) => setSelection({ scope: selectionScope, ids, allMatching: false })
  const selectAllMatching = () => setSelection({ scope: selectionScope, ids: selectedIds, allMatching: true })
  const clearSelection = () => setSelectedIds([])
  const extensions = useRecordListExtensions(recordType, schema, list.data?.data ?? EMPTY_RECORDS)
  const extensionAddon = extensions.titleAddon
  const combinedTitleAddon = useMemo(() => {
    if (!titleAddon) return extensionAddon
    if (!extensionAddon) return titleAddon
    return (record: RecordItem) => (
      <>
        {extensionAddon(record)}
        {titleAddon(record)}
      </>
    )
  }, [extensionAddon, titleAddon])
  const setArchivedMode = (mode: ArchivedMode) =>
    setSearchParams((current) => {
      const next = new URLSearchParams(current)
      if (mode === 'exclude') next.delete(ARCHIVED_PARAM)
      else next.set(ARCHIVED_PARAM, mode)
      next.delete('page')
      return next
    })

  const update = (patch: Partial<ListUrlState>, replace = false) =>
    setUrlState((current) => ({ ...current, ...patch }), { replace })

  const setFilter = (filter: FilterNode | null) => update({ filter, page: 1 })
  const setSort = (sort: SortSpec[]) => update({ sort, page: 1 })
  const setColumns = (columns: string[]) => update({ columns })
  const setDensity = (density: TableDensity) => update({ density })
  const setSearch = (search: string) => update({ search, page: 1 }, true)
  const setLayout = (next: ListLayout) => update({ layout: next, page: 1 })
  const setCalendarField = (key: string) => update({ calendarField: key })
  const clearFilters = () => update({ filter: null, search: '', page: 1 })
  const selectView = (viewId: string) =>
    setUrlState((current) => ({ view: viewId, search: '', page: 1, perPage: current.perPage }))
  const resetToView = () =>
    setUrlState((current) => ({ view: current.view, search: current.search, page: 1, perPage: current.perPage }))
  const handleSaved = (view: SavedView) =>
    setUrlState((current) => ({ view: view.id, search: current.search, page: 1, perPage: current.perPage }))

  const hasQuery = Boolean(config.filter) || urlState.search.trim() !== ''
  const isLoading = !ready || list.isPending
  const items = list.data?.data ?? EMPTY_RECORDS
  const total = list.data?.meta.total ?? 0
  const bulkContext: BulkActionContext = {
    recordType,
    selectedIds,
    allMatching,
    total: allMatching ? total : selectedIds.length,
    query: { filter: listFilter, search: urlState.search, archived: archivedMode },
    clearSelection,
  }
  const moduleBulkActions = useSlot('record.list.bulkActions', bulkContext)
  const selectable = Boolean(bulkActions) || moduleBulkActions.length > 0
  const invalidQuery = list.isError && isApiError(list.error) && list.error.status === 422

  let body: ReactNode
  if (showCalendar && calendarField) {
    body = ready ? (
      <RecordCalendar
        recordType={recordType}
        resource={resource}
        field={calendarField}
        filter={config.filter}
        search={urlState.search}
        archived={archivedMode === 'exclude' ? archived : archivedMode}
        onRecordClick={onRowClick}
      />
    ) : (
      <RecordListSkeleton />
    )
  } else if (list.isError && !list.data) {
    body = (
      <ErrorState
        title={t('states.errorTitle')}
        description={invalidQuery ? t('states.invalidQuery') : undefined}
        onRetry={() => void list.refetch()}
      />
    )
  } else if (!isLoading && items.length === 0 && hasQuery) {
    body = (
      <EmptyState
        icon={SearchX}
        title={t('states.filteredTitle')}
        description={t('states.filteredDescription')}
        action={
          <Button variant="outline" size="sm" onClick={clearFilters}>
            {t('states.clearFilters')}
          </Button>
        }
      />
    )
  } else if (!isLoading && items.length === 0 && urlState.page <= 1) {
    body = (
      <EmptyState title={t('states.emptyTitle')} description={t('states.emptyDescription')} action={emptyAction} />
    )
  } else {
    body = (
      <>
        {!isLoading && <PaginationSummary page={urlState.page} perPage={urlState.perPage} total={total} />}
        <RecordTable
          schema={schema}
          resource={resource}
          records={items}
          columns={config.columns}
          sort={config.sort}
          onSortChange={setSort}
          mobileSorting={false}
          density={config.density}
          isLoading={isLoading}
          isFetching={list.isFetching}
          selectedIds={selectable ? selectedIds : undefined}
          onSelectionChange={selectable ? setSelectedIds : undefined}
          onRowClick={onRowClick}
          rowClassName={rowClassName}
          titleAddon={combinedTitleAddon}
        />
        {!isLoading && (
          <Pagination
            page={urlState.page}
            perPage={urlState.perPage}
            total={total}
            onPageChange={(page) => update({ page })}
          />
        )}
      </>
    )
  }

  const viewSwitcher = (
    <ViewSwitcher
          recordType={recordType}
          views={views}
          activeView={activeView}
          config={config}
          isModified={isModified}
          onSelect={selectView}
          onSaved={handleSaved}
          onDiscard={resetToView}
          layout={
            showCalendar && calendarField
              ? { viewType: 'calendar', settings: { [CALENDAR_FIELD_SETTING]: calendarField.key } }
              : activeView?.view_type === 'calendar'
                ? { viewType: 'table' }
                : undefined
          }
        />
  )
  const applyMobileOptions = (options: MobileListOptions) => setSearchParams((current) => {
    const next = buildListSearch({ ...parseListSearch(current), filter: options.filter, sort: options.sort, columns: options.columns, density: options.density, layout: options.layout, calendarField: options.calendarField, perPage: options.perPage, page: 1 }, current)
    if (archived === undefined && archivedFilter) {
      if (options.archived === 'exclude') next.delete(ARCHIVED_PARAM)
      else next.set(ARCHIVED_PARAM, options.archived)
    }
    return next
  })

  return (
    <div className={cn('grid min-w-0 gap-3', className)}>
      {mobile ? <MobileRecordToolbar
        recordType={recordType}
        schema={extensions.filterSchema}
        titleField={resource.titleField}
        resetColumns={baseConfig.columns}
        view={<>{viewSwitcher}{toolbarActions}<RecordListActions recordType={recordType} /></>}
        search={urlState.search}
        onSearch={setSearch}
        options={{ filter: config.filter, sort: config.sort, columns: config.columns, density: config.density, archived: archivedMode, layout, perPage: urlState.perPage, calendarField: calendarField?.key ?? '' }}
        allowArchived={archived === undefined && archivedFilter}
        dateFields={dateFields}
        onApply={applyMobileOptions}
      /> : (
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        {viewSwitcher}
        <SearchBox value={urlState.search} onChange={setSearch} className="w-full sm:w-64" />
        <FilterMenu schema={extensions.filterSchema} filter={config.filter} onApply={setFilter} />
        <Slot name="record.list.toolbar" props={{ recordType, filter: config.filter, applyFilter: setFilter }} />
        <div className="ml-auto flex flex-wrap items-center gap-2">
          {dateFields.length > 0 && (
            <LayoutToggle
              layout={showCalendar ? 'calendar' : 'table'}
              onChange={setLayout}
              fields={dateFields.map((field) => ({ key: field.key, label: field.label }))}
              fieldKey={calendarField?.key ?? ''}
              onFieldChange={setCalendarField}
            />
          )}
          {(!showCalendar || (archived === undefined && archivedFilter)) && (
            <Popover>
              <PopoverTrigger asChild><Button variant="outline" size="sm"><SlidersHorizontal aria-hidden />{t('display.title')}</Button></PopoverTrigger>
              <PopoverContent align="end" className="grid gap-4">
              {archived === undefined && archivedFilter && (
                <NativeSelect
                  aria-label={t('archived.label')}
                  className="w-auto"
                  value={archivedMode}
                  onChange={(event) => setArchivedMode(parseArchivedParam(event.target.value))}
                >
                  <option value="exclude">{t('archived.exclude')}</option>
                  <option value="include">{t('archived.include')}</option>
                  <option value="only">{t('archived.only')}</option>
                </NativeSelect>
              )}
              {!showCalendar && <>
              <ColumnsMenu
                schema={schema}
                columns={config.columns}
                titleField={resource.titleField}
                onChange={setColumns}
                onReset={() => update({ columns: undefined })}
              />
              <DensityToggle value={config.density} onChange={setDensity} />
              <PageSizeSelect value={urlState.perPage} onChange={(perPage) => update({ perPage, page: 1 })} />
              </>}
              </PopoverContent>
            </Popover>
          )}
          <RecordListActions recordType={recordType} />
          {toolbarActions}
        </div>
      </div>
      )}
      {viewMissing && (
        <p role="status" className="text-sm text-muted-foreground">
          {t('views.notFound')}
        </p>
      )}
      {archived === undefined && archivedFilter && archivedMode !== 'exclude' && <Button variant="ghost" size="sm" className="w-fit" onClick={() => setArchivedMode('exclude')}>
        {t(archivedMode === 'only' ? 'archived.only' : 'archived.include')} · {t('states.clearFilters')}
      </Button>}
      {mobile && <FilterChips schema={extensions.filterSchema} filter={config.filter} onChange={setFilter} />}
      {!mobile && <FilterChips schema={extensions.filterSchema} filter={config.filter} onChange={setFilter} />}
      {selectable && !showCalendar && selectedIds.length > 0 && (
        <div
          role="region"
          aria-label={t('table.selected', { count: selectedIds.length })}
          className="flex flex-wrap items-center gap-2 rounded-md border bg-muted/50 px-3 py-2 text-sm"
        >
          <span className="font-medium">
            {allMatching ? t('table.allMatchingSelected', { count: total }) : t('table.selected', { count: selectedIds.length })}
          </span>
          {!allMatching && selectedIds.length === items.length && total > selectedIds.length && (
            <Button variant="link" size="sm" className="h-auto px-0" onClick={selectAllMatching}>
              {t('table.selectAllMatching', { count: total })}
            </Button>
          )}
          <div className="flex flex-wrap items-center gap-2">
            {bulkActions?.(bulkContext)}
            {moduleBulkActions.map(({ key, Component }) => (
              <Component key={key} {...bulkContext} />
            ))}
          </div>
          <Button variant="ghost" size="sm" className="ml-auto" onClick={clearSelection}>
            {t('table.clearSelection')}
          </Button>
        </div>
      )}
      {body}
    </div>
  )
}

interface LayoutToggleProps {
  layout: ListLayout
  onChange: (layout: ListLayout) => void
  fields: { key: string; label: string }[]
  fieldKey: string
  onFieldChange: (key: string) => void
}

function LayoutToggle({ layout, onChange, fields, fieldKey, onFieldChange }: LayoutToggleProps) {
  const { t } = useTranslation('calendar')
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      <SegmentedControl
        aria-label={t('layout.label')}
        size="sm"
        value={layout}
        onValueChange={onChange}
        options={[
          { value: 'table', label: t('layout.table'), icon: <Table2 aria-hidden /> },
          { value: 'calendar', label: t('layout.calendar'), icon: <CalendarDays aria-hidden /> },
        ]}
      />
      {layout === 'calendar' && fields.length > 1 && (
        <NativeSelect
          aria-label={t('layout.field')}
          className="w-auto max-w-48"
          value={fieldKey}
          onChange={(event) => onFieldChange(event.target.value)}
        >
          {fields.map((field) => (
            <option key={field.key} value={field.key}>
              {field.label}
            </option>
          ))}
        </NativeSelect>
      )}
    </div>
  )
}
