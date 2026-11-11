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

export interface AuditPage {
  data: AuditEntry[]
  meta: { total: number; page: number; per_page: number }
}

const BASE = '/console/v1'

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
  session: () => api.get<Operator>(`${BASE}/session`),
  signIn: (email: string, password: string) => api.post<Operator>(`${BASE}/session`, { email, password }),
  signOut: () => api.delete<void>(`${BASE}/session`),
  changePassword: (currentPassword: string, newPassword: string) =>
    api.post<void>(`${BASE}/session/password`, { current_password: currentPassword, new_password: newPassword }),
  operators: () => api.get<Operator[]>(`${BASE}/operators`),
  invite: (email: string, displayName: string) =>
    api.post<Invitation>(`${BASE}/operators`, { email, display_name: displayName }),
  renewInvitation: (id: string) => api.post<Invitation>(`${BASE}/operators/${id}/invitation`),
  deactivate: (id: string) => api.post<Operator>(`${BASE}/operators/${id}/deactivate`),
  invitation: (token: string) => api.get<InvitationDetails>(`${BASE}/invitations/${encodeURIComponent(token)}`),
  acceptInvitation: (token: string, password: string) =>
    api.post<Operator>(`${BASE}/invitations/${encodeURIComponent(token)}`, { password }),
  tenants: () => api.get<Tenant[]>(`${BASE}/tenants`),
  tenant: (slug: string) => api.get<Tenant>(`${BASE}/tenants/${slug}`),
  createTenant: (input: CreateTenantInput) => api.post<Tenant>(`${BASE}/tenants`, input),
  suspend: (slug: string) => api.post<Tenant>(`${BASE}/tenants/${slug}/suspend`),
  resume: (slug: string) => api.post<Tenant>(`${BASE}/tenants/${slug}/resume`),
  deleteTenant: (slug: string, exportCopy: boolean) =>
    apiRequest<Tenant>(`${BASE}/tenants/${slug}`, { method: 'DELETE', body: { confirm: slug, export: exportCopy } }),
  addHost: (slug: string, host: string) => api.post<Tenant>(`${BASE}/tenants/${slug}/hosts`, { host }),
  removeHost: (slug: string, host: string) => api.delete<Tenant>(`${BASE}/tenants/${slug}/hosts/${encodeURIComponent(host)}`),
  makePrimary: (slug: string, host: string) =>
    api.post<Tenant>(`${BASE}/tenants/${slug}/hosts/${encodeURIComponent(host)}/primary`),
  health: (slug: string) => api.get<TenantHealth>(`${BASE}/tenants/${slug}/health`),
  catalog: () => api.get<Catalog>(`${BASE}/catalog`),
  audit: (page: number) => api.get<AuditPage>(`${BASE}/audit`, { query: { page, per_page: 50 } }),
}
