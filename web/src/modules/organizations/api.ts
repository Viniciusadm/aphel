import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import { recordsKeys } from '@/features/records/api'

import type { Page } from '@/features/records/types'
import { COMPANY_RECORD_TYPE } from './resource'

export interface CompanyContactLink {
  contact: {
    id: string
    name: string
    email?: string | null
    phone?: string | null
    mobile?: string | null
    job_title?: string | null
  }
  role: string | null
  is_primary: boolean
  link_id: string | null
}

export const COMPANY_CONTACTS_PER_PAGE = 100

export const companyContactsKey = (companyId: string) =>
  [...recordsKeys.detail(COMPANY_RECORD_TYPE, companyId), 'people'] as const

export const companiesApi = {
  listContacts: (companyId: string) =>
    api.get<Page<CompanyContactLink>>(`/v1/organizations/${encodeURIComponent(companyId)}/people`, {
      query: { page: 1, per_page: COMPANY_CONTACTS_PER_PAGE },
    }),
}

export function useCompanyContacts(companyId: string) {
  return useQuery({
    queryKey: companyContactsKey(companyId),
    queryFn: () => companiesApi.listContacts(companyId),
  })
}
