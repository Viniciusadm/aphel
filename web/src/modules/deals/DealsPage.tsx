import { SegmentedControl } from '@/components/ui/segmented-control'
import { useQueryClient } from '@tanstack/react-query'
import { KanbanSquare, Plus, Table2 } from 'lucide-react'
import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useSearchParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useFieldSchema, useSavedViews } from '@/features/records/api'
import { FilterChips, FilterMenu } from '@/features/records/filters'
import { RecordFormDialog } from '@/features/records/form'
import { RecordListView, SearchBox } from '@/features/records/table'
import type { FilterNode, RecordItem, RecordViewConfig, SavedView } from '@/features/records/types'
import {
  ALL_RECORDS_VIEW,
  DEFAULT_DENSITY,
  LIST_PARAMS,
  ViewSwitcher,
  buildListSearch,
  pickDefaultView,
  useListUrlState,
} from '@/features/records/views'
import { hasExplicitQuery, viewDensity } from '@/features/records/views/urlState'
import { applyFilters } from '@/kernel/modules/filters'
import { Slot } from '@/kernel/modules/Slot'
import { cn } from '@/lib/utils'
import { activePipelines, defaultPipeline, usePipelines } from '@/modules/pipelines/api'
import type { Pipeline } from '@/modules/pipelines/types'
import { dealsKeys } from './api'
import { pipelineFromFilter, withPipelineFilter } from './pipeline-filter'
import { DealsBoard } from './board/DealsBoard'
import { DEAL_CREATE_LAYOUT, DEAL_RECORD_TYPE, dealPath } from './resource'
import { PageHeader } from '@/components/ui/page-header'
import { NativeSelect } from '@/components/ui/native-select'
import { RecordListActions } from '@/features/records/table/RecordListActions'
import { Page } from '@/components/ui/page'
import { WorkspaceOptions } from '@/components/ui/workspace-options'

export const DEALS_MODE_PARAM = 'mode'
export const DEALS_PIPELINE_PARAM = 'pipeline'

export type DealsMode = 'board' | 'table'

const KANBAN_VIEW_TYPE = 'kanban'
const PIPELINE_VIEW_SETTING = 'pipeline_id'

function isKanbanView(view: SavedView): boolean {
  return view.view_type === KANBAN_VIEW_TYPE
}

function viewPipeline(view: SavedView | undefined): string | null {
  const value = (view?.settings as Record<string, unknown> | undefined)?.[PIPELINE_VIEW_SETTING]
  return typeof value === 'string' && value ? value : null
}

function sameFilter(left: FilterNode | null | undefined, right: FilterNode | null | undefined): boolean {
  return JSON.stringify(left ?? null) === JSON.stringify(right ?? null)
}

function useBoardViews() {
  const viewsQuery = useSavedViews(DEAL_RECORD_TYPE)
  const [urlState] = useListUrlState()
  const views = useMemo(() => (viewsQuery.data ?? []).filter(isKanbanView), [viewsQuery.data])
  const requested = urlState.view && urlState.view !== ALL_RECORDS_VIEW ? urlState.view : null
  const activeView = requested ? views.find((view) => view.id === requested) : undefined
  return { viewsQuery, views, activeView }
}

function badgesAddon(record: RecordItem) {
  return <Slot name="deals.badges" props={{ deal: record, placement: 'list' }} />
}

function toneRowClassName(record: RecordItem) {
  return applyFilters('deals.tone', 'default', { deal: record }) === 'warning' ? 'bg-warning-solid/10' : undefined
}

function BoardToolbar({ filter, children }: { filter: FilterNode | null; children?: ReactNode }) {
  const schema = useFieldSchema(DEAL_RECORD_TYPE)
  const [urlState, setUrlState] = useListUrlState()
  const setFilter = (next: FilterNode | null) => setUrlState((current) => ({ ...current, filter: next, page: 1 }))
  const setSearch = (search: string) => setUrlState((current) => ({ ...current, search, page: 1 }), { replace: true })

  return (
    <div className="grid gap-2">
      <div className="flex flex-wrap items-center gap-2">
        <SearchBox value={urlState.search} onChange={setSearch} className="min-w-0 flex-1 sm:w-64 sm:flex-none" />
        {schema.data ? (
          <FilterMenu schema={schema.data} filter={filter} onApply={setFilter} />
        ) : (
          <Skeleton className="h-8 w-24" />
        )}
        {children}
        <RecordListActions recordType={DEAL_RECORD_TYPE} />
        <Slot name="record.list.toolbar" props={{ recordType: DEAL_RECORD_TYPE, filter, applyFilter: setFilter }} />
      </div>
      {schema.data && <FilterChips schema={schema.data} filter={filter} onChange={setFilter} />}
    </div>
  )
}

function PipelineFilterSelect() {
  const { t } = useTranslation('deals', { keyPrefix: 'pipeline' })
  const pipelines = usePipelines()
  const [urlState, setUrlState] = useListUrlState()
  const selected = pipelineFromFilter(urlState.filter ?? null) ?? ''
  const options = pipelines.data ?? []
  if (options.length < 2) return null

  return (
    <NativeSelect
      aria-label={t('filter.label')}
      value={selected}
      onChange={(event) =>
        setUrlState((current) => ({
          ...current,
          filter: withPipelineFilter(current.filter ?? null, event.target.value || null),
          page: 1,
        }))
      }
      className="h-8 max-w-48 px-2"
    >
      <option value="">{t('filter.all')}</option>
      {options.map((item) => (
        <option key={item.id} value={item.id}>
          {item.is_active ? item.name : t('filter.inactive', { name: item.name })}
        </option>
      ))}
    </NativeSelect>
  )
}

function useBoardPipeline(activeView?: SavedView) {
  const pipelines = usePipelines()
  const [searchParams] = useSearchParams()
  const requested = searchParams.get(DEALS_PIPELINE_PARAM) ?? viewPipeline(activeView)
  const pipeline = pipelines.data?.find((item) => item.id === requested) ?? defaultPipeline(pipelines.data)
  return { pipelines, pipeline }
}

function entryStage(pipeline: Pipeline | undefined) {
  return pipeline?.stages
    .filter((stage) => stage.is_active && stage.kind === 'open')
    .sort((left, right) => left.position - right.position)[0]
}

function BoardMode({ canMove, presentation }: { canMove: boolean; presentation: ReactNode }) {
  const { t } = useTranslation('deals')
  const { viewsQuery, views, activeView } = useBoardViews()
  const { pipelines, pipeline } = useBoardPipeline(activeView)
  const [urlState, setUrlState] = useListUrlState()
  const [searchParams, setSearchParams] = useSearchParams()
  const selectable = activePipelines(pipelines.data)
  const options = pipeline && !pipeline.is_active ? [...selectable, pipeline] : selectable
  const pipelineParam = searchParams.get(DEALS_PIPELINE_PARAM)
  const filter = urlState.filter !== undefined ? urlState.filter : (activeView?.filter ?? null)
  const defaultView = pickDefaultView(views)
  const needsDefault =
    viewsQuery.isSuccess &&
    urlState.view === null &&
    !hasExplicitQuery(urlState) &&
    pipelineParam === null &&
    Boolean(defaultView)

  useEffect(() => {
    if (needsDefault && defaultView) {
      setUrlState((current) => ({ ...current, view: defaultView.id }), { replace: true })
    }
  }, [needsDefault, defaultView, setUrlState])

  const config: RecordViewConfig = {
    filter,
    sort: activeView?.sort ?? [],
    columns: activeView?.columns ?? [],
    density: viewDensity(activeView) ?? DEFAULT_DENSITY,
  }
  const isModified =
    (urlState.filter !== undefined && !sameFilter(urlState.filter, activeView?.filter)) ||
    (pipelineParam !== null && pipelineParam !== viewPipeline(activeView))

  const openView = (viewId: string, keepSearch: boolean) =>
    setSearchParams((current) => {
      const next = buildListSearch(
        { view: viewId, search: keepSearch ? urlState.search : '', page: 1, perPage: urlState.perPage },
        current,
      )
      next.delete(DEALS_PIPELINE_PARAM)
      return next
    })

  if (pipelines.isPending) return <Skeleton className="h-96 w-full" />
  if (pipelines.isError) return <ErrorState title={t('board.loadError')} onRetry={() => void pipelines.refetch()} />

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
      <BoardToolbar filter={filter}>
        <WorkspaceOptions label={activeView?.name ?? pipeline?.name ?? t('list.mode')}>
        {presentation}
        <ViewSwitcher
          recordType={DEAL_RECORD_TYPE}
          views={views}
          activeView={activeView}
          config={config}
          isModified={isModified}
          onSelect={(viewId) => openView(viewId, false)}
          onSaved={(view) => openView(view.id, true)}
          onDiscard={() => openView(urlState.view ?? ALL_RECORDS_VIEW, true)}
          layout={{
            viewType: KANBAN_VIEW_TYPE,
            settings: pipeline ? { [PIPELINE_VIEW_SETTING]: pipeline.id } : {},
          }}
        />
        {options.length > 1 && (
          <NativeSelect
            aria-label={t('board.pipeline')}
            value={pipeline?.id ?? ''}
            onChange={(event) =>
              setSearchParams((current) => {
                const next = new URLSearchParams(current)
                next.set(DEALS_PIPELINE_PARAM, event.target.value)
                return next
              })
            }
            className="h-8 px-2"
          >
            {options.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name}
              </option>
            ))}
          </NativeSelect>
        )}
        </WorkspaceOptions>
      </BoardToolbar>
      <DealsBoard
        request={{ pipelineId: pipeline?.id, filter, search: urlState.search }}
        canMove={canMove}
      />
    </div>
  )
}

export function DealsPage() {
  const { t } = useTranslation('deals')
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { hasPermission } = useSession()
  const [searchParams, setSearchParams] = useSearchParams()
  const [creating, setCreating] = useState(false)
  const mode: DealsMode = searchParams.get(DEALS_MODE_PARAM) === 'table' ? 'table' : 'board'
  const canCreate = hasPermission('deals.create')
  const canMove = hasPermission('deals.update')
  const { views: kanbanViews, activeView: activeKanbanView } = useBoardViews()
  const { pipelines, pipeline: boardPipeline } = useBoardPipeline(activeKanbanView)
  const [listState] = useListUrlState()
  const tablePipelineId = pipelineFromFilter(listState.filter ?? null)
  const targetPipeline =
    mode === 'board' ? boardPipeline : pipelines.data?.find((item) => item.id === tablePipelineId)
  const targetStage = entryStage(targetPipeline)
  const presetValues =
    targetPipeline && targetStage ? { pipeline_id: targetPipeline.id, stage_id: targetStage.id } : undefined

  const setMode = (next: DealsMode) =>
    setSearchParams((current) => {
      const params = new URLSearchParams(current)
      params.set(DEALS_MODE_PARAM, next)
      const viewId = params.get(LIST_PARAMS.view)
      const isKanban = kanbanViews.some((view) => view.id === viewId)
      if (viewId && viewId !== ALL_RECORDS_VIEW && next === 'board' && !isKanban) params.delete(LIST_PARAMS.view)
      return params
    })

  const newButton = canCreate ? (
    <Button size="sm" onClick={() => setCreating(true)}>
      <Plus aria-hidden="true" />
      {t('list.new')}
    </Button>
  ) : undefined

  const presentation = (
          <SegmentedControl
            aria-label={t('list.mode')}
            size="sm"
            value={mode}
            onValueChange={setMode}
            options={[
              { value: 'board', label: t('list.board'), icon: <KanbanSquare aria-hidden="true" /> },
              { value: 'table', label: t('list.table'), icon: <Table2 aria-hidden="true" /> },
            ]}
          />
  )

  return (
    <Page className={cn(mode === 'board' && 'h-full')}>
<PageHeader title={t('list.title')} actions={<>
        <div className="flex flex-wrap items-center gap-2">
          {newButton}
        </div>
      
      </>} />
      {mode === 'board' ? (
        <BoardMode canMove={canMove} presentation={presentation} />
      ) : (
        <RecordListView
          recordType={DEAL_RECORD_TYPE}
          toolbarActions={
            <>
              {presentation}
              <PipelineFilterSelect />
            </>
          }
          emptyAction={newButton}
          onRowClick={(record) => navigate(dealPath(record.id))}
          rowClassName={toneRowClassName}
          titleAddon={badgesAddon}
          archivedFilter
        />
      )}
      {canCreate && (
        <RecordFormDialog
          recordType={DEAL_RECORD_TYPE}
          open={creating}
          onOpenChange={setCreating}
          layout={DEAL_CREATE_LAYOUT}
          defaultValues={presetValues}
          presetValues={presetValues}
          title={t('dialog.title')}
          description={t('dialog.description')}
          onCreated={() => void queryClient.invalidateQueries({ queryKey: dealsKeys.boards() })}
        />
      )}
    </Page>
  )
}
