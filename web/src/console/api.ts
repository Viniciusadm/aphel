import { api, apiRequest } from '@/api'
import type { ModuleDataExport, ModuleInfo, ModulesApi } from '@/kernel/modules/api'
import type { InstalledPlugin, PluginConsent, PluginPackConsent, PluginPackResult } from '@/features/settings/modules/plugins/types'
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
export type TenantModule = ModuleInfo
export type TenantPlugin = InstalledPlugin
export type TenantPluginConsent = PluginConsent
export type TenantPluginPackConsent = PluginPackConsent
export type TenantPluginPackResult = PluginPackResult
export type TenantPluginsReadiness = { state: 'loading' | 'ready' | 'degraded'; version: number | null }
export type TenantStatus = 'provisioning' | 'active' | 'suspended' | 'deleting'

export interface AuditPage {
  data: AuditEntry[]
  meta: { total: number; page: number; per_page: number }
}

const BASE = '/console/v1'

function packageForm(file: File, rolePresets: string[] = []): FormData {
  const form = new FormData()
  form.append('package', file)
  if (rolePresets.length > 0) form.append('role_presets', rolePresets.join(','))
  return form
}

function acknowledgement(acknowledgeConflicts: boolean, webVersion?: string): URLSearchParams {
  return new URLSearchParams({
    acknowledge_conflicts: String(acknowledgeConflicts),
    ...(webVersion ? { acknowledge_web_version: webVersion } : {}),
  })
}

export const consoleKeys = {
  session: ['console', 'session'] as const,
  tenants: ['console', 'tenants'] as const,
  tenant: (slug: string) => ['console', 'tenants', slug] as const,
  health: (slug: string) => ['console', 'tenants', slug, 'health'] as const,
  plugins: (slug: string) => ['console', 'tenants', slug, 'plugins'] as const,
  modules: (slug: string) => ['console', 'tenants', slug, 'modules'] as const,
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
  updateFeatures: (slug: string, pluginManagement: boolean) =>
    api.put<Tenant>(`${BASE}/tenants/${slug}/features`, { plugin_management: pluginManagement }),
  plugins: (slug: string) => api.get<TenantPlugin[]>(`${BASE}/tenants/${slug}/plugins`),
  pluginsReadiness: (slug: string) => api.get<TenantPluginsReadiness>(`${BASE}/tenants/${slug}/plugins/readiness`),
  reviewPack: (slug: string, file: File) =>
    api.post<TenantPluginPackConsent>(`${BASE}/tenants/${slug}/plugins/packs?dry_run=true`, packageForm(file)),
  installPack: (slug: string, file: File, rolePresets: string[], acknowledgeConflicts: boolean, webVersion?: string) =>
    api.post<TenantPluginPackResult>(
      `${BASE}/tenants/${slug}/plugins/packs?${acknowledgement(acknowledgeConflicts, webVersion)}`,
      packageForm(file, rolePresets),
    ),
  reapplyPluginGrants: (slug: string, key: string) =>
    api.post<TenantPlugin>(`${BASE}/tenants/${slug}/plugins/${encodeURIComponent(key)}/grants`),
  reviewPlugin: (slug: string, file: File) =>
    api.post<TenantPluginConsent>(`${BASE}/tenants/${slug}/plugins?dry_run=true`, packageForm(file)),
  installPlugin: (slug: string, file: File, rolePresets: string[], acknowledgeConflicts: boolean, webVersion?: string) =>
    api.post<TenantPlugin>(
      `${BASE}/tenants/${slug}/plugins?${acknowledgement(acknowledgeConflicts, webVersion)}`,
      packageForm(file, rolePresets),
    ),
  setPluginEnabled: (slug: string, key: string, enabled: boolean, acknowledgeConflicts = false, webVersion?: string) =>
    api.post<TenantPlugin>(
      `${BASE}/tenants/${slug}/plugins/${encodeURIComponent(key)}/${enabled ? 'enable' : 'disable'}?${acknowledgement(acknowledgeConflicts, webVersion)}`,
    ),
  uninstallPlugin: (slug: string, key: string, purge: boolean) =>
    api.delete<void>(`${BASE}/tenants/${slug}/plugins/${encodeURIComponent(key)}?purge=${purge}`),
  updatePluginSettings: (slug: string, key: string, values: Record<string, unknown>) =>
    api.put<TenantPlugin>(`${BASE}/tenants/${slug}/plugins/${encodeURIComponent(key)}/settings`, values),
  modules: (slug: string) => api.get<TenantModule[]>(`${BASE}/tenants/${slug}/modules`),
  setModuleEnabled: (slug: string, key: string, enabled: boolean) =>
    api.post<TenantModule>(`${BASE}/tenants/${slug}/modules/${encodeURIComponent(key)}/${enabled ? 'enable' : 'disable'}`),
  health: (slug: string) => api.get<TenantHealth>(`${BASE}/tenants/${slug}/health`),
  catalog: () => api.get<Catalog>(`${BASE}/catalog`),
  audit: (page: number) => api.get<AuditPage>(`${BASE}/audit`, { query: { page, per_page: 50 } }),
}

export function tenantModulesApi(slug: string): ModulesApi {
  const base = `${BASE}/tenants/${slug}/modules`
  return {
    enable: (key) => api.post<ModuleInfo>(`${base}/${encodeURIComponent(key)}/enable`),
    disable: (key) => api.post<ModuleInfo>(`${base}/${encodeURIComponent(key)}/disable`),
    updateSettings: (key, settings) => api.put<ModuleInfo>(`${base}/${encodeURIComponent(key)}/settings`, { settings }),
    exportData: (key) => api.post<ModuleDataExport>(`${base}/${encodeURIComponent(key)}/data-export`),
    purgeData: (key, confirm) => api.post<ModuleInfo>(`${base}/${encodeURIComponent(key)}/data-purge`, { confirm }),
  }
}
