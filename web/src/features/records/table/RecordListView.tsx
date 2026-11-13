import { RecordListActions } from './RecordListActions'
import { MobileRecordToolbar, type MobileListOptions } from './MobileRecordToolbar'
import { useMediaQuery } from '../calendar/useMediaQuery'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { CalendarDays, List, SearchX, SlidersHorizontal, Table2 } from 'lucide-react'
import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { useIsModuleActive } from '@/kernel/modules/registry'
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
import {useSavedViews} from '@/sdk'
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
import {useListUrlState} from '@/sdk'
import {ALL_RECORDS_VIEW, hasExplicitQuery, pickDefaultView, resolveConfig, sameConfig, viewConfig, type ListUrlState, buildListSearch, parseListSearch} from '@/sdk'
import {ViewSwitcher} from '@/sdk'
import { ColumnsMenu } from './ColumnsMenu'
import { defaultColumnKeys } from './columns'
import { DensityToggle } from './DensityToggle'
import { PageSizeSelect } from '@/components/ui/page-size-select'
import { PaginationSummary } from '@/components/ui/pagination-summary'
import { Pagination } from '@/components/ui/pagination'
import { RecordCardList } from './RecordCardList'
import { sortToParam } from './query'
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

export interface RecordListPreset {
  key: string
  label: string
  filter: FilterNode | null
  sort?: SortSpec[]
  emptyTitle?: string
  emptyDescription?: string
}

const PRESET_PARAM = 'preset'

export interface RecordListViewProps {
  recordType: string
  presets?: RecordListPreset[]
  defaultPreset?: string
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
  presets,
  defaultPreset,
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
  const calendarElsewhere = useIsModuleActive(resource.calendarLink?.module ?? '') && resource.calendarLink !== undefined
  const dateFields = useMemo(() => (calendarElsewhere ? [] : calendarFields(schema)), [schema, calendarElsewhere])
  const viewCalendar = viewCalendarConfig(activeView)
  const hasListLayout = Boolean(resource.listItem)
  const listSorts = resource.listSorts?.() ?? []
  const allowedLayout = (candidate: ListLayout | undefined): candidate is ListLayout =>
    candidate === 'table' || (candidate === 'calendar' && dateFields.length > 0) || (candidate === 'list' && hasListLayout)
  const calendarBase: ListLayout = dateFields.length > 0 ? viewCalendar.layout : 'table'
  const baseLayout: ListLayout = activeView ? calendarBase : hasListLayout && resource.defaultLayout === 'list' ? 'list' : calendarBase
  const baseCalendarField = pickCalendarField(dateFields, viewCalendar.field)
  const layout: ListLayout = allowedLayout(urlState.layout) ? urlState.layout : baseLayout
  const showList = layout === 'list'
  const calendarField = pickCalendarField(dateFields, urlState.calendarField ?? viewCalendar.field)
  const showCalendar = layout === 'calendar' && calendarField !== null
  const layoutModified =
    layout !== baseLayout || (layout === 'calendar' && calendarField?.key !== baseCalendarField?.key)
  const isModified = !sameConfig(config, baseConfig) || layoutModified
  const presetKey = searchParams.get(PRESET_PARAM) ?? defaultPreset ?? presets?.[0]?.key
  const activePreset = presets?.find((preset) => preset.key === presetKey) ?? presets?.[0]
  const presetFilter = activePreset?.filter ?? null
  const listFilter = useMemo<FilterNode | null>(() => {
    const parts = [presetFilter, baseFilter ?? null, config.filter].filter((part): part is FilterNode => part !== null)
    if (parts.length === 0) return null
    if (parts.length === 1) return parts[0]
    return { op: 'and', conditions: parts }
  }, [presetFilter, baseFilter, config.filter])
  const sort = urlState.sort ?? (activeView ? config.sort : (activePreset?.sort ?? config.sort))
  const ready = !viewsQuery.isPending && !needsDefault

  const list = useRecordList(
    recordType,
    {
      page: urlState.page,
      perPage: urlState.perPage,
      filter: listFilter,
      sort,
      search: urlState.search,
      archived: archivedMode === 'exclude' ? archived : archivedMode,
    },
    { enabled: ready && !showCalendar },
  )

  const selectionScope = JSON.stringify([requestedView, listFilter, urlState.search, archivedMode, activePreset?.key])
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
  const setPreset = (key: string) =>
    setSearchParams((current) => {
      const next = new URLSearchParams(current)
      if (key === (defaultPreset ?? presets?.[0]?.key)) next.delete(PRESET_PARAM)
      else next.set(PRESET_PARAM, key)
      next.delete('page')
      return next
    })
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
        fieldOptions={dateFields.map((field) => ({ key: field.key, label: field.label }))}
        onFieldChange={setCalendarField}
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
      <EmptyState
        title={activePreset?.emptyTitle ?? t('states.emptyTitle')}
        description={activePreset?.emptyDescription ?? t('states.emptyDescription')}
        action={emptyAction}
      />
    )
  } else {
    body = (
      <>
        {!isLoading && <PaginationSummary page={urlState.page} perPage={urlState.perPage} total={total} />}
        {showList ? (
          <RecordCardList resource={resource} records={items} isLoading={isLoading} sort={sort} />
        ) : (
        <RecordTable
          schema={schema}
          resource={resource}
          records={items}
          columns={config.columns}
          sort={sort}
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
        )}
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

  const presetSwitcher = presets && presets.length > 1 && activePreset && (
    <SegmentedControl
      aria-label={t('presets.label')}
      size="sm"
      value={activePreset.key}
      onValueChange={setPreset}
      options={presets.map((preset) => ({ value: preset.key, label: preset.label }))}
    />
  )
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
        view={<>{presetSwitcher}{viewSwitcher}{toolbarActions}<RecordListActions recordType={recordType} /></>}
        listLayout={hasListLayout}
        search={urlState.search}
        onSearch={setSearch}
        options={{ filter: config.filter, sort: config.sort, columns: config.columns, density: config.density, archived: archivedMode, layout, perPage: urlState.perPage, calendarField: calendarField?.key ?? '' }}
        allowArchived={archived === undefined && archivedFilter}
        dateFields={dateFields}
        onApply={applyMobileOptions}
      /> : (
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        {presetSwitcher}
        {viewSwitcher}
        <SearchBox value={urlState.search} onChange={setSearch} className="w-full sm:w-64" />
        <FilterMenu schema={extensions.filterSchema} filter={config.filter} onApply={setFilter} />
        <Slot name="record.list.toolbar" props={{ recordType, filter: config.filter, applyFilter: setFilter }} />
        <div className="ml-auto flex flex-wrap items-center gap-2">
          {calendarElsewhere && resource.calendarLink && (
            <Button variant="outline" size="sm" asChild>
              <Link to={resource.calendarLink.to}>
                <CalendarDays aria-hidden />
                {t('calendarLink')}
              </Link>
            </Button>
          )}
          {(dateFields.length > 0 || hasListLayout) && (
            <LayoutToggle
              layout={showCalendar ? 'calendar' : showList ? 'list' : 'table'}
              onChange={setLayout}
              withList={hasListLayout}
              withCalendar={dateFields.length > 0}
            />
          )}
          <Popover>
              <PopoverTrigger asChild><Button variant="outline" size="sm" disabled={showCalendar && !(archived === undefined && archivedFilter)}><SlidersHorizontal aria-hidden />{t('display.title')}</Button></PopoverTrigger>
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
              {showList && listSorts.length > 0 && (
                <NativeSelect
                  aria-label={t('sort.label')}
                  className="w-auto"
                  value={listSorts.find((option) => sortToParam(option.sort) === sortToParam(sort))?.key ?? ''}
                  onChange={(event) => {
                    const option = listSorts.find((item) => item.key === event.target.value)
                    if (option) update({ sort: option.sort, page: 1 })
                  }}
                >
                  {!listSorts.some((option) => sortToParam(option.sort) === sortToParam(sort)) && (
                    <option value="" disabled>
                      {t('sort.label')}
                    </option>
                  )}
                  {listSorts.map((option) => (
                    <option key={option.key} value={option.key}>
                      {option.label}
                    </option>
                  ))}
                </NativeSelect>
              )}
              {!showCalendar && !showList && <>
              <ColumnsMenu
                schema={schema}
                columns={config.columns}
                titleField={resource.titleField}
                onChange={setColumns}
                onReset={() => update({ columns: undefined })}
              />
              <DensityToggle value={config.density} onChange={setDensity} />
              </>}
              {!showCalendar && <PageSizeSelect value={urlState.perPage} onChange={(perPage) => update({ perPage, page: 1 })} />}
              </PopoverContent>
          </Popover>
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
  withList: boolean
  withCalendar: boolean
}

function LayoutToggle({ layout, onChange, withList, withCalendar }: LayoutToggleProps) {
  const { t } = useTranslation('records-calendar')
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      <SegmentedControl
        aria-label={t('layout.label')}
        size="sm"
        value={layout}
        onValueChange={onChange}
        options={[
          ...(withList ? [{ value: 'list' as const, label: t('layout.list'), icon: <List aria-hidden /> }] : []),
          { value: 'table' as const, label: t('layout.table'), icon: <Table2 aria-hidden /> },
          ...(withCalendar ? [{ value: 'calendar' as const, label: t('layout.calendar'), icon: <CalendarDays aria-hidden /> }] : []),
        ]}
      />
    </div>
  )
}
