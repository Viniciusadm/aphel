import { useTranslation } from 'react-i18next'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useJobStats } from './api'
import { useDurationFormat } from './format'
import { SectionCard } from './SectionCard'

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
          <div className="relative overflow-x-auto rounded-lg border">
            <table className="w-full text-sm">
              <caption className="sr-only">{t('queue.byKind')}</caption>
              <thead className="bg-muted/50 text-left text-muted-foreground">
                <tr>
                  <th scope="col" className="px-3 py-2 font-medium">
                    {t('queue.columns.kind')}
                  </th>
                  <th scope="col" className="hidden px-3 py-2 font-medium sm:table-cell">
                    {t('queue.columns.queue')}
                  </th>
                  {STATES.map((state) => (
                    <th key={state} scope="col" className="px-3 py-2 text-right font-medium">
                      {t(`queue.${state}`)}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {kinds.map((kind) => (
                  <tr key={`${kind.queue}:${kind.kind}`} className="border-t">
                    <td className="min-w-36 px-3 py-2 font-mono text-xs break-all">{kind.kind}</td>
                    <td className="hidden px-3 py-2 sm:table-cell">{kind.queue}</td>
                    {STATES.map((state) => (
                      <td key={state} className="px-3 py-2 text-right tabular-nums">
                        {kind[state]}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </>
    )
  }

  return <SectionCard title={t('queue.title')}>{body}</SectionCard>
}
