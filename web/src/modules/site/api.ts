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
  customers_required: boolean
  default_language: string
  available_languages: string[]
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
}
