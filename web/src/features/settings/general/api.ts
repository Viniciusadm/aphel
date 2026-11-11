import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'

export type StorageDriver = 'local' | 's3'
export type CatalogMode = 'builtin' | 'remote' | 'off'

export interface StorageSettings {
  driver: StorageDriver
  path?: string | null
  bucket?: string | null
  region?: string | null
  endpoint?: string | null
  access_key_id?: string | null
  allow_http: boolean
  force_path_style: boolean
}

export interface WebhookDelivery {
  require_https: boolean
  timeout_secs: number
  disable_after_failures: number
  retention_days: number
}

export interface ApiTokenLimits {
  limit: number
  window_secs: number
}

export interface CatalogSource {
  mode: CatalogMode
  index_url?: string | null
  allow_http: boolean
  timeout_secs: number
}

export interface MailSettings {
  host: string | null
  port: number
  from: string | null
  username: string | null
  security: string
}

export interface SiteLink {
  id: string
  label: string
  url: string
}

export interface SiteMenuEntry {
  item: string
  hidden: boolean
}

export interface SiteSettings {
  indexing: boolean
  accounts: boolean
  theme: string | null
  tagline: string | null
  menu: SiteMenuEntry[]
  languages: string[]
  links: SiteLink[]
  home?: string | null
}

export interface OrganizationConfiguration {
  public_base_url: string | null
  effective_public_base_url: string
  app_base_path: string
  app_base_url: string
  site_base_path: string
  site_host: string | null
  site_base_url: string
  site_available: boolean
  site?: SiteSettings
  secure_cookies: boolean
  max_upload_mb: number
  storage: StorageSettings
  storage_secret_configured: boolean
  webhooks: WebhookDelivery
  api_tokens: ApiTokenLimits
  catalog: CatalogSource
  mail: MailSettings
  mail_secret_configured: boolean
  mail_configured: boolean
}

export interface UpdateOrganizationConfiguration {
  mail?: MailSettings
  mail_password?: string | null
  public_base_url?: string
  app_base_path?: string
  site_base_path?: string
  site_host?: string | null
  site?: SiteSettings
  max_upload_mb?: number
  storage?: StorageSettings
  storage_secret_access_key?: string | null
  webhooks?: WebhookDelivery
  api_tokens?: ApiTokenLimits
  catalog?: CatalogSource
}

export interface StorageTestResult {
  ok: boolean
  message: string | null
}

const CONFIGURATION_PATH = '/v1/organization/settings'

export const configurationKeys = {
  all: ['organization', 'configuration'] as const,
}

export function fetchConfiguration() {
  return api.get<OrganizationConfiguration>(CONFIGURATION_PATH)
}

export function useOrganizationConfiguration(enabled = true) {
  return useQuery({ queryKey: configurationKeys.all, queryFn: fetchConfiguration, enabled })
}

export function updateConfiguration(body: UpdateOrganizationConfiguration) {
  return api.patch<OrganizationConfiguration>(CONFIGURATION_PATH, body)
}

export function testStorage(storage: StorageSettings, secretAccessKey: string | null) {
  return api.post<StorageTestResult>(`${CONFIGURATION_PATH}/storage/test`, {
    storage,
    secret_access_key: secretAccessKey,
  })
}

export function useSaveConfiguration() {
  const queryClient = useQueryClient()
  return async (body: UpdateOrganizationConfiguration) => {
    const saved = await updateConfiguration(body)
    queryClient.setQueryData(configurationKeys.all, saved)
    return saved
  }
}

export function positiveInteger(value: string, min: number, max: number): boolean {
  if (!/^\d+$/.test(value.trim())) return false
  const parsed = Number(value)
  return parsed >= min && parsed <= max
}
