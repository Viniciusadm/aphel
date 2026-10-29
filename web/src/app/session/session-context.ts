import { createContext, useContext } from 'react'
import type { components } from '@/api'

export type Me = components['schemas']['MeResponse']
export type RecordScope = components['schemas']['RecordScopeDto']

export interface SessionContextValue {
  me: Me
  hasPermission: (permission: string) => boolean
  permissionScope: (permission: string) => RecordScope | null
  logout: () => Promise<void>
  logoutAll?: () => Promise<void>
}

export const SessionContext = createContext<SessionContextValue | null>(null)

export function useSession(): SessionContextValue {
  const value = useContext(SessionContext)
  if (!value) throw new Error('useSession must be used inside SessionProvider')
  return value
}
