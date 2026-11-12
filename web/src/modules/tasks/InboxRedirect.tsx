import { Navigate, useSearchParams } from 'react-router'
import { TASKS_PATH } from '@/sdk'

export function InboxRedirect() {
  const [params] = useSearchParams()
  const next = new URLSearchParams()
  const owner = params.get('owner')
  if (owner === 'team') next.set('preset', 'team')
  else if (owner === 'all') next.set('preset', 'open')
  if (params.get('sort') === 'priority') next.set('sort', '-priority,due_at')
  const search = next.toString()
  return <Navigate to={search ? `${TASKS_PATH}?${search}` : TASKS_PATH} replace />
}
