import { Skeleton } from '@/components/ui/skeleton'

export function ReportSkeleton() {
  return (
    <div className="grid gap-4" aria-busy="true">
      <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
        <Skeleton className="h-20" />
        <Skeleton className="h-20" />
        <Skeleton className="h-20" />
        <Skeleton className="h-20" />
      </div>
      <Skeleton className="h-72" />
      <Skeleton className="h-48" />
    </div>
  )
}
