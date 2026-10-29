import { useContext } from 'react'
import { SessionContext, type Me } from '@/app/session/session-context'

export interface OfflineOwner {
  userId: string
}

let currentOwner: OfflineOwner | null = null

export function ownerFromMe(me: Me | null | undefined): OfflineOwner | null {
  if (!me) return null
  return { userId: me.user.id }
}

export function ownerKey(owner: OfflineOwner): string {
  return owner.userId
}

export function setCurrentOwner(owner: OfflineOwner | null): void {
  currentOwner = owner
}

export function getCurrentOwner(): OfflineOwner | null {
  return currentOwner
}

export function useOfflineOwner(): OfflineOwner | null {
  const session = useContext(SessionContext)
  return ownerFromMe(session?.me)
}
