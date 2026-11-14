import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type Integration = Schemas['IntegrationResponse']
export type IntegrationField = Schemas['IntegrationFieldResponse']
export type IntegrationTestResult = Schemas['IntegrationTestResponse']
export type ConfigureIntegrationBody = Schemas['ConfigureIntegrationRequest']
export type IntegrationStatus = 'not_configured' | 'inactive' | 'active' | 'error' | 'available'
export type IntegrationConfig = Record<string, unknown>

export const TEST_EMAIL_PROVIDERS = ['smtp']

export const integrationsKeys = {
  all: ['integrations'] as const,
  list: () => [...integrationsKeys.all, 'list'] as const,
}

const path = (key: string) => `/v1/integrations/${encodeURIComponent(key)}`

export const integrationsApi = {
  list: () => api.get<Integration[]>('/v1/integrations'),
  configure: (key: string, config: IntegrationConfig, clearSecrets: string[], version: string | null) =>
    api.put<Integration>(
      path(key),
      { config, clear_secrets: clearSecrets },
      { headers: version ? { 'If-Match': version } : undefined },
    ),
  remove: (key: string) => api.delete<void>(path(key)),
  test: (key: string, sendTestEmail: boolean) =>
    api.post<IntegrationTestResult>(`${path(key)}/test`, { send_test_email: sendTestEmail }),
  activate: (key: string) => api.post<Integration>(`${path(key)}/activate`),
  deactivate: (key: string) => api.post<Integration>(`${path(key)}/deactivate`),
}

export function useIntegrations() {
  return useQuery({ queryKey: integrationsKeys.list(), queryFn: integrationsApi.list })
}

export function useInvalidateIntegrations() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: integrationsKeys.all })
}

export function integrationStatus(integration: Integration): IntegrationStatus {
  const status = integration.status as IntegrationStatus
  return ['not_configured', 'inactive', 'active', 'error', 'available'].includes(status) ? status : 'inactive'
}

export function configOf(integration: Integration): IntegrationConfig {
  return integration.config as IntegrationConfig
}

export function isSecretConfigured(integration: Integration, key: string): boolean {
  return (integration.secrets as Record<string, boolean> | undefined)?.[key] === true
}
