import { useTranslation } from 'react-i18next'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useRecentFailures } from './api'
import { useDateTimeFormat } from './format'
import { SectionCard } from './SectionCard'
import type { Failure, RecentFailures } from './types'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

const TABS = ['automations', 'webhooks', 'integrations'] as const
const DEFAULT_WINDOW_DAYS = 7

function FailureTable({ failures }: { failures: Failure[] }) {
  const { t } = useTranslation('system')
  const format = useDateTimeFormat()
  if (failures.length === 0) return <p className="text-sm text-muted-foreground">{t('failures.empty')}</p>
  const showStatus = failures.some((failure) => typeof failure.response_status === 'number')

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead scope="col">
            {t('failures.columns.kind')}
          </TableHead>
          <TableHead scope="col">
            {t('failures.columns.error')}
          </TableHead>
          {showStatus && (
            <TableHead scope="col" className="hidden sm:table-cell">
              {t('failures.columns.status')}
            </TableHead>
          )}
          <TableHead scope="col" className="hidden whitespace-nowrap sm:table-cell">
            {t('failures.columns.occurredAt')}
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {failures.map((failure, index) => (
          <TableRow key={`${failure.reference_id}:${failure.occurred_at}:${index}`} className="align-top">
            <TableCell>
              <span className="font-mono text-xs break-all">{failure.kind}</span>
            </TableCell>
            <TableCell className="max-w-xs">
              <span className="line-clamp-3 text-xs break-words text-muted-foreground" title={failure.error ?? undefined}>
                {failure.error ?? '—'}
              </span>
            </TableCell>
            {showStatus && (
              <TableCell className="hidden tabular-nums sm:table-cell">{failure.response_status ?? '—'}</TableCell>
            )}
            <TableCell className="hidden whitespace-nowrap text-muted-foreground sm:table-cell">
              <time dateTime={failure.occurred_at}>{format.format(new Date(failure.occurred_at))}</time>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
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
