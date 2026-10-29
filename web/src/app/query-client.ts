import { QueryCache, QueryClient } from '@tanstack/react-query'
import { isApiError } from '@/api'
import { SESSION_QUERY_KEY } from './session/session-query'

const MAX_RETRIES = 2

function shouldRetry(failureCount: number, error: unknown): boolean {
  if (isApiError(error) && error.status >= 400 && error.status < 500) return false
  return failureCount < MAX_RETRIES
}

export function createQueryClient(): QueryClient {
  const client: QueryClient = new QueryClient({
    queryCache: new QueryCache({
      onError: (error, query) => {
        if (!isApiError(error) || error.status !== 401) return
        if (query.queryKey[0] === SESSION_QUERY_KEY[0]) return
        client.setQueryData(SESSION_QUERY_KEY, null)
      },
    }),
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        refetchOnWindowFocus: false,
        retry: shouldRetry,
      },
      mutations: {
        retry: false,
      },
    },
  })
  return client
}
