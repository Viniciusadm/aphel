import { Navigate } from 'react-router'
import { Skeleton } from '@/components/ui/skeleton'
import { useRecordTypeOptions } from './lib/record-types'

export function DefaultRecordTypeRedirect() {
  const [first] = useRecordTypeOptions()
  if (!first) return <Skeleton className="h-9 w-48" />
  return <Navigate to={first.value} replace />
}
