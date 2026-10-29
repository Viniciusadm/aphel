import type { ReactNode } from 'react'
import { useSession } from '@/app/session/session-context'
import { NoAccess } from './NoAccess'

interface RequirePermissionProps {
  permission: string
  children: ReactNode
}

export function RequirePermission({ permission, children }: RequirePermissionProps) {
  const { hasPermission } = useSession()
  if (!hasPermission(permission)) return <NoAccess />
  return <>{children}</>
}
