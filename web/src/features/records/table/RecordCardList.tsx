import { Skeleton } from '@/components/ui/skeleton'
import { useRecordFormatting } from '../formatting'
import type { RecordResource } from '../resources'
import type { RecordItem, SortSpec } from '../types'

interface RecordCardListProps {
  resource: RecordResource
  records: RecordItem[]
  isLoading: boolean
  sort: SortSpec[]
}

export function RecordCardList({ resource, records, isLoading, sort }: RecordCardListProps) {
  const formatting = useRecordFormatting()
  const Item = resource.listItem
  if (!Item) return null

  if (isLoading && records.length === 0) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
      </div>
    )
  }

  const groups = resource.listGroups
    ? resource.listGroups(records, { timezone: formatting.timezone, sort })
    : [{ key: 'all', title: '', items: records }]

  return (
    <div className="grid gap-6">
      {groups.map((group) => (
        <section key={group.key} className="grid gap-2" aria-label={group.title || undefined}>
          {group.title && (
            <h2 className="text-sm font-semibold text-muted-foreground">
              {group.title} <span className="font-normal">({group.items.length})</span>
            </h2>
          )}
          <ul className="grid gap-2">
            {group.items.map((record) => (
              <Item key={record.id} record={record} />
            ))}
          </ul>
        </section>
      ))}
    </div>
  )
}
