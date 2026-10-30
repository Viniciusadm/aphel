import { Card, CardContent, CardHeader } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'

export function SetupLoadingCard({ label }: { label: string }) {
  return (
    <Card aria-busy="true">
      <span role="status" className="sr-only">
        {label}
      </span>
      <CardHeader className="gap-2">
        <Skeleton className="h-7 w-2/3" />
        <Skeleton className="h-4 w-full" />
      </CardHeader>
      <CardContent className="grid gap-3">
        <Skeleton className="h-16 w-full" />
        <Skeleton className="h-16 w-full" />
        <Skeleton className="h-16 w-full" />
      </CardContent>
    </Card>
  )
}
