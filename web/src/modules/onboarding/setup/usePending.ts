import { useSession } from '@/app/session/session-context'
import { SETTINGS_MANAGE } from '@/sdk'
import { useOnboardingState } from '../api'

export function useOnboardingPending(): boolean | undefined {
  const { hasPermission } = useSession()
  const canManage = hasPermission(SETTINGS_MANAGE)
  const state = useOnboardingState(canManage)
  if (!canManage) return false
  if (state.isError) return false
  return state.data ? state.data.status === 'pending' : undefined
}
