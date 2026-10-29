import { Navigate, useLocation } from 'react-router'
import { useSession } from '@/app/session/session-context'

const TWO_FACTOR_SETTINGS_PATH = '/account/security'

export function TwoFactorGate() {
  const { me } = useSession()
  const { pathname } = useLocation()
  const blocked = me.two_factor.required && !me.two_factor.enabled
  if (!blocked || pathname === TWO_FACTOR_SETTINGS_PATH) return null
  return <Navigate to={TWO_FACTOR_SETTINGS_PATH} replace />
}
