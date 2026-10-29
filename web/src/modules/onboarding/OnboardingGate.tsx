import { useEffect } from 'react'
import { useLocation, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { useOnboardingState } from './api'
import { markPrompted, wasPrompted } from './lib/prompt'

const EXEMPT_PATHS = ['/onboarding', '/data/onboarding']

export function OnboardingGate() {
  const { hasPermission } = useSession()
  const canManage = hasPermission(SETTINGS_MANAGE)
  if (!canManage) return null
  return <PendingOnboardingRedirect />
}

function PendingOnboardingRedirect() {
  const { pathname } = useLocation()
  const navigate = useNavigate()
  const exempt = EXEMPT_PATHS.some((path) => pathname === path || pathname.startsWith(`${path}/`))
  const { data } = useOnboardingState(!wasPrompted())

  useEffect(() => {
    if (exempt || data?.status !== 'pending' || wasPrompted()) return
    markPrompted()
    void navigate('/onboarding')
  }, [data?.status, exempt, navigate])

  return null
}
