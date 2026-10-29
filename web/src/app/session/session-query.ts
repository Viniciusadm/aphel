import { useQuery, type Query, type QueryClient } from '@tanstack/react-query'
import { fetchSession } from '@/features/auth/api'
import type { Me } from './session-context'

export const SESSION_QUERY_KEY = ['session', 'me'] as const

function isOtherQuery(query: Query): boolean {
  return query.queryKey[0] !== SESSION_QUERY_KEY[0]
}

export function useSessionQuery() {
  return useQuery({ queryKey: SESSION_QUERY_KEY, queryFn: fetchSession })
}

export function storeSession(queryClient: QueryClient, me: Me): void {
  queryClient.removeQueries({ predicate: isOtherQuery })
  queryClient.setQueryData(SESSION_QUERY_KEY, me)
}

export function clearSession(queryClient: QueryClient): void {
  queryClient.removeQueries({ predicate: isOtherQuery })
  queryClient.setQueryData(SESSION_QUERY_KEY, null)
}

export function refreshSession(queryClient: QueryClient): Promise<void> {
  return queryClient.invalidateQueries({ queryKey: SESSION_QUERY_KEY })
}
