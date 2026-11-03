import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'
type Schemas = components['schemas']
export type AmountMode = Schemas['AmountMode']
export type LineItemSettings = Schemas['LineItemSettingsResponse']
export type UpdateLineItemSettings = Schemas['UpdateLineItemSettingsRequest']

export const DEAL_AMOUNT_MODES: AmountMode[] = ['manual', 'items']
export const settingsKeys = { settings: () => ['line-items', 'settings'] as const }
export const settingsApi = {
  dealItemSettings: () => api.get<LineItemSettings>('/v1/line-items/settings'),
  updateLineItemSettings: (body: UpdateLineItemSettings, version?: string | null) =>
    api.patch<LineItemSettings>('/v1/line-items/settings', body, {
      headers: version ? { 'If-Match': version } : undefined,
    }),
}
export function useLineItemSettings() { return useQuery({ queryKey: settingsKeys.settings(), queryFn: settingsApi.dealItemSettings }) }
