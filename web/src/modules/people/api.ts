import { CONTACT_RECORD_TYPE } from './resource'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import { type RecordsPage as Page, recordsKeys } from '@/sdk'
import { type ContactCompanyRole } from '@/sdk'

export interface ContactCompanyLink {
  company: { id: string; name: string; domain?: string | null }
  role: ContactCompanyRole | string | null
  is_primary: boolean
  link_id: string | null
  created_at?: string | null
}

export interface LinkCompanyRequest {
  company_id: string
  role?: string | null
}

export const CONTACT_COMPANIES_PER_PAGE = 100

function companiesPath(contactId: string, companyId?: string) {
  const base = `/v1/people/${encodeURIComponent(contactId)}/organizations`
  return companyId ? `${base}/${encodeURIComponent(companyId)}` : base
}

export const contactCompaniesKey = (contactId: string) =>
  [...recordsKeys.detail(CONTACT_RECORD_TYPE, contactId), 'organizations'] as const

export const contactsApi = {
  listCompanies: (contactId: string) =>
    api.get<Page<ContactCompanyLink>>(companiesPath(contactId), {
      query: { page: 1, per_page: CONTACT_COMPANIES_PER_PAGE },
    }),
  linkCompany: (contactId: string, request: LinkCompanyRequest) =>
    api.post<ContactCompanyLink>(companiesPath(contactId), request),
  unlinkCompany: (contactId: string, companyId: string) => api.delete<void>(companiesPath(contactId, companyId)),
}

export function useContactCompanies(contactId: string) {
  return useQuery({
    queryKey: contactCompaniesKey(contactId),
    queryFn: () => contactsApi.listCompanies(contactId),
  })
}

export function useContactCompanyMutations(contactId: string) {
  const queryClient = useQueryClient()
  const refresh = () => {
    void queryClient.invalidateQueries({ queryKey: contactCompaniesKey(contactId) })
    void queryClient.invalidateQueries({ queryKey: [...recordsKeys.all, 'company'] })
  }
  const link = useMutation({
    mutationFn: (request: LinkCompanyRequest) => contactsApi.linkCompany(contactId, request),
    onSuccess: refresh,
  })
  const unlink = useMutation({
    mutationFn: (companyId: string) => contactsApi.unlinkCompany(contactId, companyId),
    onSuccess: refresh,
  })
  return { link, unlink }
}
