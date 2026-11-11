import { api, apiRequest } from '@/api'
import type { components } from './api-types'

type Schemas = components['schemas']

export type Operator = Schemas['OperatorResponse']
export type Invitation = Schemas['InvitationResponse']
export type InvitationDetails = Schemas['InvitationDetailsResponse']
export type Tenant = Schemas['TenantResponse']
export type TenantHealth = Schemas['TenantHealthResponse']
export type Catalog = Schemas['CatalogResponse']
export type AuditEntry = Schemas['AuditEntryResponse']
export type CreateTenantInput = Schemas['CreateTenantRequest']
export type TenantStatus = 'provisioning' | 'active' | 'suspended' | 'deleting'

interface Data<T> {
  data: T
}

export interface AuditPage {
  data: AuditEntry[]
  meta: { total: number; page: number; per_page: number }
}

const BASE = '/console/v1'

async function data<T>(request: Promise<Data<T>>): Promise<T> {
  return (await request).data
}

export const consoleKeys = {
  session: ['console', 'session'] as const,
  tenants: ['console', 'tenants'] as const,
  tenant: (slug: string) => ['console', 'tenants', slug] as const,
  health: (slug: string) => ['console', 'tenants', slug, 'health'] as const,
  catalog: ['console', 'catalog'] as const,
  operators: ['console', 'operators'] as const,
  audit: (page: number) => ['console', 'audit', page] as const,
  invitation: (token: string) => ['console', 'invitation', token] as const,
}

export const consoleApi = {
  session: () => data(api.get<Data<Operator>>(`${BASE}/session`)),
  signIn: (email: string, password: string) =>
    data(api.post<Data<Operator>>(`${BASE}/session`, { email, password })),
  signOut: () => api.delete<void>(`${BASE}/session`),
  changePassword: (currentPassword: string, newPassword: string) =>
    api.post<void>(`${BASE}/session/password`, { current_password: currentPassword, new_password: newPassword }),
  operators: () => data(api.get<Data<Operator[]>>(`${BASE}/operators`)),
  invite: (email: string, displayName: string) =>
    data(api.post<Data<Invitation>>(`${BASE}/operators`, { email, display_name: displayName })),
  renewInvitation: (id: string) => data(api.post<Data<Invitation>>(`${BASE}/operators/${id}/invitation`)),
  deactivate: (id: string) => data(api.post<Data<Operator>>(`${BASE}/operators/${id}/deactivate`)),
  invitation: (token: string) => data(api.get<Data<InvitationDetails>>(`${BASE}/invitations/${encodeURIComponent(token)}`)),
  acceptInvitation: (token: string, password: string) =>
    data(api.post<Data<Operator>>(`${BASE}/invitations/${encodeURIComponent(token)}`, { password })),
  tenants: () => data(api.get<Data<Tenant[]>>(`${BASE}/tenants`)),
  tenant: (slug: string) => data(api.get<Data<Tenant>>(`${BASE}/tenants/${slug}`)),
  createTenant: (input: CreateTenantInput) => data(api.post<Data<Tenant>>(`${BASE}/tenants`, input)),
  suspend: (slug: string) => data(api.post<Data<Tenant>>(`${BASE}/tenants/${slug}/suspend`)),
  resume: (slug: string) => data(api.post<Data<Tenant>>(`${BASE}/tenants/${slug}/resume`)),
  deleteTenant: (slug: string, exportCopy: boolean) =>
    data(
      apiRequest<Data<Tenant>>(`${BASE}/tenants/${slug}`, {
        method: 'DELETE',
        body: { confirm: slug, export: exportCopy },
      }),
    ),
  addHost: (slug: string, host: string) => data(api.post<Data<Tenant>>(`${BASE}/tenants/${slug}/hosts`, { host })),
  removeHost: (slug: string, host: string) =>
    data(api.delete<Data<Tenant>>(`${BASE}/tenants/${slug}/hosts/${encodeURIComponent(host)}`)),
  makePrimary: (slug: string, host: string) =>
    data(api.post<Data<Tenant>>(`${BASE}/tenants/${slug}/hosts/${encodeURIComponent(host)}/primary`)),
  health: (slug: string) => data(api.get<Data<TenantHealth>>(`${BASE}/tenants/${slug}/health`)),
  catalog: () => data(api.get<Data<Catalog>>(`${BASE}/catalog`)),
  audit: (page: number) => api.get<AuditPage>(`${BASE}/audit`, { query: { page, per_page: 50 } }),
}
