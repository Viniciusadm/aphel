import { useQuery } from '@tanstack/react-query'
import { consoleApi, consoleKeys } from './api'

export function useConsoleSession() {
  return useQuery({ queryKey: consoleKeys.session, queryFn: consoleApi.session, retry: false, staleTime: 60_000 })
}
