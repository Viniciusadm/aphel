import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api, type components } from '@/api'
import { useSession } from '@/app/session/session-context'
import { recordsApi, recordsKeys } from '../api'
import type {
  CreateViewRequest,
  RecordViewConfig,
  SavedView,
  TableDensity,
  UpdateViewRequest,
  ViewType,
} from '../types'

export const SETTINGS_MANAGE = 'settings.manage'
export const MAX_VIEW_NAME_LENGTH = 140

export type TeamReference = components['schemas']['TeamReferenceResponse']

type ViewSettings = SavedView['settings']

export interface ViewLayoutOptions {
  viewType?: ViewType
  settings?: Record<string, unknown>
}

export function viewSettings(
  density: TableDensity,
  current?: ViewSettings,
  extra?: Record<string, unknown>,
): ViewSettings {
  return { ...(current ?? {}), ...(extra ?? {}), density } as unknown as ViewSettings
}

export function viewConfigRequest(config: RecordViewConfig, current?: SavedView, layout?: ViewLayoutOptions) {
  return {
    filter: config.filter,
    sort: config.sort.map((spec) => ({ field: spec.field, direction: spec.direction ?? 'asc' })),
    columns: config.columns,
    settings: viewSettings(config.density, current?.settings, layout?.settings),
    ...(layout?.viewType ? { view_type: layout.viewType } : {}),
  } satisfies UpdateViewRequest
}

export function useViewMutations(recordType: string) {
  const queryClient = useQueryClient()
  const key = recordsKeys.views(recordType)
  const invalidate = () => queryClient.invalidateQueries({ queryKey: key })
  const store = (saved: SavedView) => {
    queryClient.setQueryData<SavedView[]>(key, (current) => {
      if (!current) return current
      return current.some((view) => view.id === saved.id)
        ? current.map((view) => (view.id === saved.id ? saved : view))
        : [...current, saved]
    })
    return invalidate()
  }

  const create = useMutation({
    mutationFn: (request: CreateViewRequest) => recordsApi.createView(recordType, request),
    onSuccess: store,
  })
  const update = useMutation({
    mutationFn: ({ viewId, request }: { viewId: string; request: UpdateViewRequest }) => {
      const current = queryClient.getQueryData<SavedView[]>(key)?.find((view) => view.id === viewId)
      return recordsApi.updateView(recordType, viewId, request, current?.updated_at)
    },
    onSuccess: store,
  })
  const remove = useMutation({
    mutationFn: (viewId: string) => recordsApi.deleteView(recordType, viewId),
    onSuccess: invalidate,
  })
  const userDefault = useMutation({
    mutationFn: async ({ viewId, enabled }: { viewId: string; enabled: boolean }) => {
      if (enabled) await recordsApi.setUserDefaultView(recordType, viewId)
      else await recordsApi.clearUserDefaultView(recordType, viewId)
    },
    onSuccess: invalidate,
  })

  return { create, update, remove, userDefault }
}

const TEAMS_PER_PAGE = 100
const MAX_TEAM_PAGES = 50

async function listAllTeams(): Promise<TeamReference[]> {
  const teams: TeamReference[] = []
  for (let page = 1; page <= MAX_TEAM_PAGES; page += 1) {
    const result = await api.get<{ data: TeamReference[]; meta: { total: number } }>('/v1/teams', {
      query: { page, per_page: TEAMS_PER_PAGE },
    })
    teams.push(...result.data.map((team) => ({ id: team.id, name: team.name })))
    if (result.data.length === 0 || page * TEAMS_PER_PAGE >= result.meta.total) break
  }
  return teams
}

function isTeamReference(value: unknown): value is TeamReference {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as { id?: unknown }).id === 'string' &&
    typeof (value as { name?: unknown }).name === 'string'
  )
}

export function sessionTeams(me: object): TeamReference[] {
  const teams = (me as { teams?: unknown }).teams
  return Array.isArray(teams) ? teams.filter(isTeamReference).map((team) => ({ id: team.id, name: team.name })) : []
}

export function useViewTeams(enabled: boolean) {
  const { me, hasPermission } = useSession()
  const canManage = hasPermission(SETTINGS_MANAGE)
  const ownTeams = sessionTeams(me)
  const all = useQuery({
    queryKey: [...recordsKeys.all, 'view-teams'],
    queryFn: listAllTeams,
    enabled: enabled && canManage,
    staleTime: 5 * 60_000,
  })
  if (canManage) return { data: all.data, isSuccess: all.isSuccess, isError: all.isError, canShare: true }
  return { data: ownTeams, isSuccess: true, isError: ownTeams.length === 0, canShare: ownTeams.length > 0 }
}
