import { useTranslation } from 'react-i18next'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useRecentFailures } from './api'
import { useDateTimeFormat } from './format'
import { SectionCard } from './SectionCard'
import type { Failure, RecentFailures } from './types'

const TABS = ['automations', 'webhooks', 'integrations'] as const
const DEFAULT_WINDOW_DAYS = 7

function FailureTable({ failures }: { failures: Failure[] }) {
  const { t } = useTranslation('system')
  const format = useDateTimeFormat()
  if (failures.length === 0) return <p className="text-sm text-muted-foreground">{t('failures.empty')}</p>
  const showStatus = failures.some((failure) => typeof failure.response_status === 'number')

  return (
    <div className="relative overflow-x-auto rounded-lg border">
      <table className="w-full text-sm">
        <thead className="bg-muted/50 text-left text-muted-foreground">
          <tr>
            <th scope="col" className="px-3 py-2 font-medium">
              {t('failures.columns.kind')}
            </th>
            <th scope="col" className="px-3 py-2 font-medium">
              {t('failures.columns.error')}
            </th>
            {showStatus && (
              <th scope="col" className="hidden px-3 py-2 font-medium sm:table-cell">
                {t('failures.columns.status')}
              </th>
            )}
            <th scope="col" className="hidden px-3 py-2 font-medium whitespace-nowrap sm:table-cell">
              {t('failures.columns.occurredAt')}
            </th>
          </tr>
        </thead>
        <tbody>
          {failures.map((failure, index) => (
            <tr key={`${failure.reference_id}:${failure.occurred_at}:${index}`} className="border-t align-top">
              <td className="px-3 py-2">
                <span className="font-mono text-xs break-all">{failure.kind}</span>
              </td>
              <td className="max-w-xs px-3 py-2">
                <span className="line-clamp-3 text-xs break-words text-muted-foreground" title={failure.error ?? undefined}>
                  {failure.error ?? '—'}
                </span>
              </td>
              {showStatus && (
                <td className="hidden px-3 py-2 tabular-nums sm:table-cell">{failure.response_status ?? '—'}</td>
              )}
              <td className="hidden px-3 py-2 whitespace-nowrap text-muted-foreground sm:table-cell">
                <time dateTime={failure.occurred_at}>{format.format(new Date(failure.occurred_at))}</time>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function FailureTabs({ failures }: { failures: RecentFailures }) {
  const { t } = useTranslation('system')
  return (
    <Tabs defaultValue="automations" className="grid gap-3">
      <TabsList aria-label={t('failures.title')}>
        {TABS.map((tab) => (
          <TabsTrigger key={tab} value={tab} className="h-7">
            {t(`failures.tabs.${tab}`)} ({failures[tab].length})
          </TabsTrigger>
        ))}
      </TabsList>
      {TABS.map((tab) => (
        <TabsContent key={tab} value={tab}>
          <FailureTable failures={failures[tab]} />
        </TabsContent>
      ))}
    </Tabs>
  )
}

export function FailuresSection() {
  const { t } = useTranslation('system')
  const failures = useRecentFailures()

  let body
  if (failures.isPending) {
    body = <Skeleton className="h-32 w-full" />
  } else if (failures.isError) {
    body = <ErrorState onRetry={() => void failures.refetch()} />
  } else {
    body = <FailureTabs failures={failures.data} />
  }

  return (
    <SectionCard
      title={t('failures.title')}
      description={t('failures.description', { days: failures.data?.window_days ?? DEFAULT_WINDOW_DAYS })}
    >
      {body}
    </SectionCard>
  )
}
