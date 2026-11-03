import { Building2, CalendarCheck, FileText, Handshake, Package, User } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import type { SearchGroup, SearchHit } from '@/modules/search/ui/api'
import { searchHitPath, useInternalRecordTypes, useSearchRecordTypeLabel } from '@/modules/search/ui/links'

export interface SearchRecordTypeIconProps {
  recordType: string
  className?: string
}

export function SearchRecordTypeIcon({ recordType, className }: SearchRecordTypeIconProps) {
  const props = { className, 'aria-hidden': true }
  switch (recordType) {
    case 'people.person':
      return <User {...props} />
    case 'organizations.organization':
      return <Building2 {...props} />
    case 'deals.deal':
      return <Handshake {...props} />
    case 'tasks.task':
      return <CalendarCheck {...props} />
    case 'products.product':
      return <Package {...props} />
    default:
      return <FileText {...props} />
  }
}

interface SearchHitRowProps {
  hit: SearchHit
  onSelect?: (hit: SearchHit) => void
}

function SearchHitRow({ hit, onSelect }: SearchHitRowProps) {
  const path = searchHitPath(hit)
  const content = (
    <>
      <span className="flex size-8 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
        <SearchRecordTypeIcon recordType={hit.record_type} className="size-4" />
      </span>
      <span className="grid min-w-0 flex-1 gap-0.5">
        <span className="truncate text-sm font-medium">{hit.title}</span>
        {hit.subtitle && <span className="truncate text-xs text-muted-foreground">{hit.subtitle}</span>}
      </span>
    </>
  )
  const className = 'flex min-w-0 items-center gap-3 rounded-md px-3 py-2'
  if (!path) return <div className={className}>{content}</div>
  return (
    <Link
      to={path}
      onClick={() => onSelect?.(hit)}
      className={cn(className, 'transition-colors hover:bg-accent focus-visible:bg-accent focus-visible:outline-none')}
    >
      {content}
    </Link>
  )
}

export interface SearchResultGroupsProps {
  groups: SearchGroup[]
  onSelect?: (hit: SearchHit) => void
  className?: string
}

export function SearchResultGroups({ groups, onSelect, className }: SearchResultGroupsProps) {
  const { t } = useTranslation('search')
  const label = useSearchRecordTypeLabel()
  const internal = useInternalRecordTypes()
  return (
    <div className={cn('grid gap-4', className)}>
      {groups.filter((group) => !internal.has(group.record_type)).map((group) => {
        const headingId = `search-group-${group.record_type}`
        return (
          <section
            key={group.record_type}
            aria-labelledby={headingId}
            className="min-w-0 rounded-lg border bg-card text-card-foreground"
          >
            <header className="flex items-center justify-between gap-2 border-b px-3 py-2">
              <h2 id={headingId} className="truncate text-sm font-semibold">
                {label(group.record_type, true)}
              </h2>
              <Badge variant="secondary" aria-label={t('results.total', { count: group.total })}>
                {group.total}
              </Badge>
            </header>
            <ul className="grid gap-0.5 p-1">
              {group.items.map((hit) => (
                <li key={hit.id} className="min-w-0">
                  <SearchHitRow hit={hit} onSelect={onSelect} />
                </li>
              ))}
            </ul>
            {group.total > group.items.length && (
              <p className="border-t px-3 py-2 text-xs text-muted-foreground">
                {t('results.more', { shown: group.items.length, count: group.total })}
              </p>
            )}
          </section>
        )
      })}
    </div>
  )
}
