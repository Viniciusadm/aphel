import { useQuery } from '@tanstack/react-query'
import { api, isApiError, type components } from '@/api'

export type InstallRequest = components['schemas']['InstallRequest'] & { public_base_url?: string | null }
export type InstallerMe = components['schemas']['MeResponse']

const INSTALLER_PATH = '/v1/installer'

export const INSTALLER_STATUS_KEY = ['installer', 'status'] as const

export async function fetchInstallerAvailable(): Promise<boolean> {
  try {
    const status = await api.get<components['schemas']['InstallerStatusResponse']>(INSTALLER_PATH)
    return status.available
  } catch (error) {
    if (isApiError(error) && error.status === 404) return false
    throw error
  }
}

export function useInstallerAvailable() {
  return useQuery({
    queryKey: INSTALLER_STATUS_KEY,
    queryFn: fetchInstallerAvailable,
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  })
}

export function install(body: InstallRequest) {
  return api.post<InstallerMe>(INSTALLER_PATH, body)
}

export interface RestoreResult {
  restored: boolean
}

export function restoreBackup(file: File) {
  const body = new FormData()
  body.append('file', file)
  return api.post<RestoreResult>(`${INSTALLER_PATH}/restore`, body)
}
