import { useTranslation } from 'react-i18next'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useJobStats } from './api'
import { useDurationFormat } from './format'
import { SectionCard } from './SectionCard'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

const STATES = ['pending', 'scheduled', 'running', 'failed'] as const

export function QueueSection() {
  const { t } = useTranslation('system')
  const duration = useDurationFormat()
  const stats = useJobStats()

  let body
  if (stats.isPending) {
    body = <Skeleton className="h-32 w-full" />
  } else if (stats.isError) {
    body = <ErrorState onRetry={() => void stats.refetch()} />
  } else {
    const kinds = stats.data.kinds.filter((kind) => STATES.some((state) => kind[state] > 0))
    body = (
      <>
        <dl className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {STATES.map((state) => (
            <div key={state} className="grid gap-1 rounded-lg border px-3 py-2">
              <dt className="text-xs text-muted-foreground">{t(`queue.${state}`)}</dt>
              <dd className="text-2xl font-semibold tabular-nums">{stats.data[state]}</dd>
            </div>
          ))}
        </dl>
        {typeof stats.data.oldest_pending_age_seconds === 'number' && (
          <p className="text-sm text-muted-foreground">
            {t('queue.oldest', { age: duration(stats.data.oldest_pending_age_seconds) })}
          </p>
        )}
        {kinds.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t('queue.empty')}</p>
        ) : (
          <Table>
            <caption className="sr-only">{t('queue.byKind')}</caption>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">
                  {t('queue.columns.kind')}
                </TableHead>
                <TableHead scope="col" className="hidden sm:table-cell">
                  {t('queue.columns.queue')}
                </TableHead>
                {STATES.map((state) => (
                  <TableHead key={state} scope="col" className="text-right">
                    {t(`queue.${state}`)}
                  </TableHead>
                ))}
              </TableRow>
            </TableHeader>
            <TableBody>
              {kinds.map((kind) => (
                <TableRow key={`${kind.queue}:${kind.kind}`}>
                  <TableCell className="min-w-36 font-mono text-xs break-all">{kind.kind}</TableCell>
                  <TableCell className="hidden sm:table-cell">{kind.queue}</TableCell>
                  {STATES.map((state) => (
                    <TableCell key={state} className="text-right tabular-nums">
                      {kind[state]}
                    </TableCell>
                  ))}
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </>
    )
  }

  return <SectionCard title={t('queue.title')}>{body}</SectionCard>
}
