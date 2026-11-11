import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'

export interface SiteTheme {
  key: string
  name: string
  description: string | null
}

export interface SiteMenuItem {
  id: string
  label: string
  path: string
}

export interface SiteOverview {
  base_url: string
  themes: SiteTheme[]
  active_theme: string | null
  menu_items: SiteMenuItem[]
  pages: number
}

export interface SiteRedirect {
  id: number
  record_type: string
  field: string
  old_value: string
  new_value: string
  old_path: string | null
  new_path: string | null
  created_at: string
}

const SITE_PATH = '/v1/site'

export const siteKeys = {
  all: ['site'] as const,
  overview: ['site', 'overview'] as const,
  redirects: ['site', 'redirects'] as const,
}

export function useSiteOverview() {
  return useQuery({ queryKey: siteKeys.overview, queryFn: () => api.get<SiteOverview>(SITE_PATH) })
}

export function useSiteRedirects() {
  return useQuery({ queryKey: siteKeys.redirects, queryFn: () => api.get<SiteRedirect[]>(`${SITE_PATH}/redirects`) })
}

export function useInvalidateSite() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: siteKeys.all })
}

export const siteApi = {
  deleteRedirect: (id: number) => api.delete<void>(`${SITE_PATH}/redirects/${id}`),
  purgeCache: () => api.post<void>(`${SITE_PATH}/cache/purge`),
}
