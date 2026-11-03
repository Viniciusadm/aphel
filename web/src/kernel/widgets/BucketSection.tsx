import { Badge } from '@/components/ui/badge'
import type { LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'
interface Bucket<T> { count: number; items: T[] }
interface SectionProps<T> {
  id: string
  icon: LucideIcon
  title: string
  bucket: Bucket<T>
  tone?: 'danger' | 'default'
  emptyLabel: string
  moreLabel: (count: number) => string
  renderItem: (item: T) => ReactNode
}

export function BucketSection<T extends { id: string }>({ id, icon: Icon, title, bucket, tone = 'default', renderItem, emptyLabel, moreLabel }: SectionProps<T>) {

  const headingId = `${id}-heading`
  const remaining = bucket.count - bucket.items.length
  return (
    <section aria-labelledby={headingId} className="flex min-w-0 flex-col gap-2">
      <h4 id={headingId} className="flex items-center gap-2 text-sm font-medium">
        <Icon className={cn('size-4', tone === 'danger' && bucket.count > 0 ? 'text-destructive' : 'text-muted-foreground')} aria-hidden />
        <span className="truncate">{title}</span>
        <Badge count tone={tone === 'danger' && bucket.count > 0 ? 'destructive' : 'neutral'} className="ml-auto">
          {bucket.count}
        </Badge>
      </h4>
      {bucket.count === 0 ? (
        <p className="text-xs text-muted-foreground">{emptyLabel}</p>
      ) : (
        <ul className="grid gap-1.5">
          {bucket.items.map((item) => (
            <li key={item.id} className="min-w-0 rounded-md border px-3 py-2 text-sm">
              {renderItem(item)}
            </li>
          ))}
          {remaining > 0 && (
            <li className="text-xs text-muted-foreground">{moreLabel(remaining)}</li>
          )}
        </ul>
      )}
    </section>
  )
}

