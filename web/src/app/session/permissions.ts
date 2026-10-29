import type { Me, RecordScope } from './session-context'

const scopeRank: Record<RecordScope, number> = { own: 0, team: 1, organization: 2 }

export function permissionScopeOf(me: Me, permission: string): RecordScope | null {
  let widest: RecordScope | null = null
  for (const grant of me.permissions) {
    if (grant.permission !== permission) continue
    if (widest === null || scopeRank[grant.scope] > scopeRank[widest]) widest = grant.scope
  }
  return widest
}

export function hasPermissionOf(me: Me, permission: string): boolean {
  return me.permissions.some((grant) => grant.permission === permission)
}
