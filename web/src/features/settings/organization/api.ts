import { useQuery } from '@tanstack/react-query'
import { API_BASE_URL, api, type components } from '@/api'
import { uploadFile } from '@/features/files/api'

export type OrganizationSettings = components['schemas']['OrganizationResponse']
export type UpdateOrganizationRequest = components['schemas']['UpdateOrganizationRequest']

export const ORGANIZATION_PATH = '/v1/organization'
export const MAX_LOGO_BYTES = 2 * 1024 * 1024
export const LOGO_MIME_TYPES = ['image/png', 'image/jpeg', 'image/webp', 'image/gif']

export const organizationKeys = {
  settings: ['organization', 'settings'] as const,
}

export function fetchOrganization() {
  return api.get<OrganizationSettings>(ORGANIZATION_PATH)
}

export function useOrganizationSettings() {
  return useQuery({ queryKey: organizationKeys.settings, queryFn: fetchOrganization })
}

export function updateOrganization(body: UpdateOrganizationRequest) {
  return api.patch<OrganizationSettings>(ORGANIZATION_PATH, body)
}

export async function uploadLogo(file: File): Promise<string> {
  const uploaded = await uploadFile({ file })
  return uploaded.file.id
}

export function organizationLogoUrl(logoFileId: string): string {
  return `${API_BASE_URL}${ORGANIZATION_PATH}/logo?v=${encodeURIComponent(logoFileId)}`
}
