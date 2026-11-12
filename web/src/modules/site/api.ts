import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { SiteNavigationItem } from '@/sdk'

export interface SiteTheme {
  key: string
  name: string
  description: string | null
}

export interface SiteOverview {
  base_url: string
  themes: SiteTheme[]
  active_theme: string | null
  pages: number
  customers_required: boolean
  default_language: string
  available_languages: string[]
  people_available: boolean
}

export interface SiteStatus {
  base_url: string
  indexing: boolean
  home: { label: string; url: string } | null
  menu_items: number
  named: boolean
  has_logo: boolean
  languages: number
  content: { record_type: string; label: string; published: number; drafts: number }[]
  recent: { record_type: string; record_id: string; title: string; url: string; published_at: string | null }[]
}

export interface SiteTarget {
  target: string
  label: string
  group: string
  path: string
}

export interface SiteCustomer {
  id: string
  name: string
  email: string
  email_verified_at: string | null
  last_login_at: string | null
  created_at: string
}

interface CustomerPage {
  data: SiteCustomer[]
  meta: { total: number }
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

export interface SitePathRedirect {
  id: number
  source_path: string
  target: string
  permanent: boolean
  created_at: string
}

export interface NewSitePathRedirect {
  source_path: string
  target: string
  permanent: boolean
}

interface SitePreview {
  url: string
  expires_in_minutes: number
}

const SITE_PATH = '/v1/site'

export const siteKeys = {
  all: ['site'] as const,
  overview: ['site', 'overview'] as const,
  redirects: ['site', 'redirects'] as const,
  pathRedirects: ['site', 'path-redirects'] as const,
}

export function useSiteOverview() {
  return useQuery({ queryKey: siteKeys.overview, queryFn: () => api.get<SiteOverview>(SITE_PATH) })
}

export interface SiteStructure {
  home: string | null
  menu: string[]
}

export function saveNavigation(navigation: SiteNavigationItem[], home: string | null) {
  return api.put<SiteStructure>(`${SITE_PATH}/navigation`, { navigation, home })
}

export function useSiteStatus() {
  return useQuery({ queryKey: [...siteKeys.all, 'status'], queryFn: () => api.get<SiteStatus>(`${SITE_PATH}/status`) })
}

export function useSiteTargets() {
  return useQuery({ queryKey: [...siteKeys.all, 'targets'], queryFn: () => api.get<SiteTarget[]>(`${SITE_PATH}/targets`) })
}

export function useSiteRedirects() {
  return useQuery({ queryKey: siteKeys.redirects, queryFn: () => api.get<SiteRedirect[]>(`${SITE_PATH}/redirects`) })
}

export function useSitePathRedirects() {
  return useQuery({
    queryKey: siteKeys.pathRedirects,
    queryFn: () => api.get<SitePathRedirect[]>(`${SITE_PATH}/path-redirects`),
  })
}

export function useSiteCustomers(search: string) {
  return useQuery({
    queryKey: [...siteKeys.all, 'customers', search],
    queryFn: () =>
      api.get<CustomerPage>('/v1/entities/site.customer/records', {
        query: { sort: '-created_at', per_page: 20, search: search || undefined },
      }),
  })
}

export function useInvalidateSite() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: siteKeys.all })
}

export const siteApi = {
  deleteRedirect: (id: number) => api.delete<void>(`${SITE_PATH}/redirects/${id}`),
  purgeCache: () => api.post<void>(`${SITE_PATH}/cache/purge`),
  createPathRedirect: (redirect: NewSitePathRedirect) =>
    api.post<SitePathRedirect>(`${SITE_PATH}/path-redirects`, redirect),
  deletePathRedirect: (id: number) => api.delete<void>(`${SITE_PATH}/path-redirects/${id}`),
  startPreview: (theme: string | null) => api.post<SitePreview>(`${SITE_PATH}/preview`, { theme }),
}
