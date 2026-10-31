import { Flame, Lock } from 'lucide-react'
import { Link } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { LEAD_PRO_KEY, READ_PERMISSION, useHotLeads } from './api'
import { TemperatureBadge } from './TemperatureBadge'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'

export function BoardPage() {
  const t = useModuleTranslation(LEAD_PRO_KEY)
  const { hasPermission } = useSession()
  const allowed = hasPermission(READ_PERMISSION)
  const board = useHotLeads()

  if (!allowed) return <EmptyState icon={Lock} title={t('board.noAccess')} description={t('board.noAccessDescription')} />

  return (
    <Page>
      <PageHeader
        title={t('board.title')}
        description={board.data ? t('board.description', { threshold: board.data.hot_threshold }) : t('board.loading')}
      />
      {board.isPending ? (
        <Skeleton className="h-40 w-full" />
      ) : board.isError ? (
        <ErrorState onRetry={() => void board.refetch()} />
      ) : board.data.data.length === 0 ? (
        <EmptyState icon={Flame} title={t('board.empty')} description={t('board.emptyDescription')} />
      ) : (
        <ul className="grid gap-2">
          {board.data.data.map((lead) => (
            <li key={lead.contact_id} className="flex items-center justify-between gap-3 rounded-md border bg-card px-4 py-3">
              <div className="grid min-w-0">
                <Link to={`/contacts/${lead.contact_id}`} className="truncate font-medium text-primary hover:underline">
                  {lead.name}
                </Link>
                {lead.email && <span className="truncate text-sm text-muted-foreground">{lead.email}</span>}
              </div>
              {lead.temperature !== null && (
                <TemperatureBadge temperature={lead.temperature} hotThreshold={board.data.hot_threshold} />
              )}
            </li>
          ))}
        </ul>
      )}
    </Page>
  )
}
