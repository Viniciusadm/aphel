import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import { periodKey, periodQuery } from './period'
import type {
  DashboardLayout,
  DashboardSubjects,
  SaveLayoutRequest,
  WidgetCatalogPage,
  WidgetDataResponse,
  WidgetScope,
  WidgetSettings,
} from './types'

export const dashboardKeys = {
  all: ['dashboard'] as const,
  catalog: () => [...dashboardKeys.all, 'catalog'] as const,
  layout: () => [...dashboardKeys.all, 'layout'] as const,
  subjects: () => [...dashboardKeys.all, 'subjects'] as const,
  data: () => [...dashboardKeys.all, 'data'] as const,
  widget: (key: string, scope: WidgetScope, settings: WidgetSettings) =>
    [
      ...dashboardKeys.data(),
      key,
      scope.period ? periodKey(scope.period) : null,
      scope.subject ? `${scope.subject.type}:${scope.subject.id}` : null,
      scope.compare,
      JSON.stringify(settings),
    ] as const,
}

function widgetPath(key: string) {
  return `/v1/dashboard/widgets/${encodeURIComponent(key)}/data`
}

export const dashboardApi = {
  catalog: () => api.get<WidgetCatalogPage>('/v1/dashboard/widgets', { query: { per_page: 100 } }),
  layout: () => api.get<DashboardLayout>('/v1/dashboard/layout'),
  saveLayout: (layout: SaveLayoutRequest) => api.put<DashboardLayout>('/v1/dashboard/layout', layout),
  resetLayout: () => api.delete<DashboardLayout>('/v1/dashboard/layout'),
  subjects: () => api.get<DashboardSubjects>('/v1/dashboard/subjects'),
  widgetData: (key: string, scope: WidgetScope, settings: WidgetSettings) =>
    api.get<WidgetDataResponse>(widgetPath(key), {
      query: {
        ...(scope.period ? periodQuery(scope.period) : {}),
        user_id: scope.subject?.type === 'user' ? scope.subject.id : undefined,
        team_id: scope.subject?.type === 'team' ? scope.subject.id : undefined,
        compare_to_previous: scope.compare && scope.period ? true : undefined,
        settings: Object.keys(settings).length > 0 ? JSON.stringify(settings) : undefined,
      },
    }),
}

export function useWidgetCatalog() {
  return useQuery({
    queryKey: dashboardKeys.catalog(),
    queryFn: dashboardApi.catalog,
    select: (page) => page.data,
    staleTime: 5 * 60 * 1000,
  })
}

export function useDashboardLayout() {
  return useQuery({ queryKey: dashboardKeys.layout(), queryFn: dashboardApi.layout })
}

export function useDashboardSubjects() {
  return useQuery({ queryKey: dashboardKeys.subjects(), queryFn: dashboardApi.subjects, staleTime: 5 * 60 * 1000 })
}

export function useWidgetData(key: string, scope: WidgetScope, settings: WidgetSettings) {
  return useQuery({
    queryKey: dashboardKeys.widget(key, scope, settings),
    queryFn: () => dashboardApi.widgetData(key, scope, settings),
    placeholderData: keepPreviousData,
    staleTime: 30 * 1000,
  })
}
