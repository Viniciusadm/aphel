import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Download, RefreshCw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { api } from '@/api'
import type { components } from '@/api/schema'
import { ConfirmDialog } from '@/components/feedback/ConfirmDialog'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { SectionCard } from './SectionCard'

type UpdateStatus = components['schemas']['SystemUpdateStatus']
const PHASES = ['preparing', 'prepared', 'backing_up', 'installing', 'verifying', 'succeeded', 'failed', 'recovery_required'] as const
const MESSAGES = ['updater_interrupted', 'restore_backup_required', 'restart_failed', 'insufficient_disk', 'preparation_failed', 'backup_failed', 'invalid_image', 'invalid_version', 'invalid_databases', 'unsupported_storage_mount', 'version_mismatch'] as const

export function UpdatesSection({ consoleMode = false }: { consoleMode?: boolean }) {
  const { t } = useTranslation('updates')
  const [confirm, setConfirm] = useState(false)
  const [reconnecting, setReconnecting] = useState(false)
  const client = useQueryClient()
  const base = consoleMode ? '/console/v1/updates' : '/v1/system/updates'
  const key = ['system-updates', consoleMode] as const
  const query = useQuery({
    queryKey: key,
    queryFn: () => api.get<UpdateStatus>(base),
    refetchInterval: 5000,
    retry: 2,
    retryDelay: 3000,
    refetchIntervalInBackground: true,
  })
  const action = useMutation({
    mutationFn: ({ name, operationId }: { name: 'check' | 'prepare' | 'apply'; operationId?: string }) =>
      api.post<UpdateStatus>(`${base}/${name}`, name === 'apply' ? { operation_id: operationId, confirm: true } : {}),
    onSuccess: (data) => {
      client.setQueryData(key, data)
      setReconnecting(false)
    },
    onError: (_error, variables) => {
      if (variables.name === 'apply') setReconnecting(true)
    },
    onSettled: () => { void client.invalidateQueries({ queryKey: key }) },
  })

  if (query.data?.managed_by_console) return null
  const data = query.data
  const operation = data?.operation
  const phase = operation?.phase
  const knownPhase = PHASES.find((value) => value === phase)
  const message = MESSAGES.find((value) => value === operation?.message_code)
  const busy = phase === 'preparing' || phase === 'backing_up' || phase === 'installing' || phase === 'verifying'
  const updating = phase === 'backing_up' || phase === 'installing' || phase === 'verifying'
  const newer = data?.latest && data.latest.version.split('.').map(Number).some((part, index, all) => {
    const current = data.current_version.split('.').map(Number)
    return part > current[index] && all.slice(0, index).every((value, i) => value === current[i])
  })
  const prepared = phase === 'prepared'
  const reconnectingNow = (reconnecting || updating) && query.isError

  return (
    <SectionCard title={t('title')} description={consoleMode ? t('multiDescription') : t('description')}>
      {!data && query.isPending && <Skeleton className="h-24 w-full" />}
      {!data && query.isError && <ErrorState onRetry={() => void query.refetch()} />}
      {data && <>
        <div className="flex flex-wrap gap-x-8 gap-y-2 text-sm">
          <p>{t('installed', { version: data.current_version })}</p>
          {data.latest && <p>{t('latest', { version: data.latest.version })}</p>}
          {data.checked_at && <p>{t('checkedAt', { time: new Date(data.checked_at).toLocaleString() })}</p>}
        </div>
        {!data.enabled ? <p className="text-sm text-muted-foreground">{t('unavailable')}</p> : <>
          {!data.latest && <p className="text-sm text-muted-foreground">{data.checked_at ? t('noRelease') : t('notChecked')}</p>}
          {data.latest && !newer && !busy && !prepared && <p className="text-sm text-muted-foreground">{t('upToDate')}</p>}
          {data.latest && <details className="rounded-md border border-border p-3 text-sm">
            <summary className="cursor-pointer font-medium">{t('releaseNotes')}</summary>
            <p className="mt-3 whitespace-pre-wrap break-words text-muted-foreground">{data.latest.notes || t('noNotes')}</p>
            <a className="mt-3 inline-block underline" href={data.latest.url} target="_blank" rel="noreferrer">{t('viewRelease')}</a>
          </details>}
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" disabled={busy || action.isPending || phase === 'recovery_required'} onClick={() => action.mutate({ name: 'check' })}>
              <RefreshCw aria-hidden />{t('check')}
            </Button>
            {newer && !prepared && <Button size="sm" disabled={busy || action.isPending || phase === 'recovery_required'} onClick={() => action.mutate({ name: 'prepare' })}>
              <Download aria-hidden />{t('prepare')}
            </Button>}
            {prepared && <Button size="sm" disabled={action.isPending || reconnectingNow} onClick={() => setConfirm(true)}>{t('apply')}</Button>}
          </div>
        </>}
        <div role="status" aria-live="polite" className="grid gap-2 text-sm">
          {knownPhase && <p className="font-medium">{t(`phases.${knownPhase}`)}</p>}
          {prepared && <p>{t('preparedVersion', { version: operation?.version })}</p>}
          {message && <p className="text-muted-foreground">{t(`messages.${message}`)}</p>}
          {reconnectingNow && <p>{t('reconnecting')}</p>}
          {action.isError && !reconnectingNow && <p className="text-destructive">{t('actionFailed')}</p>}
          {query.isError && !reconnectingNow && <p className="text-destructive">{t('statusFailed')}</p>}
        </div>
        {data.history.length > 0 && <details className="text-sm">
          <summary className="cursor-pointer font-medium">{t('history')}</summary>
          <ul className="mt-3 divide-y divide-border">
            {data.history.map((entry) => {
              const entryPhase = PHASES.find((value) => value === entry.phase)
              return <li key={`${entry.id}-${entry.finished_at}`} className="grid gap-1 py-3">
                <span>{t('historyEntry', { version: entry.version, actor: entry.actor })}</span>
                <span className="text-muted-foreground">{entryPhase ? t(`phases.${entryPhase}`) : t('unknownStatus')} · {entry.finished_at ? new Date(entry.finished_at).toLocaleString() : ''}</span>
              </li>
            })}
          </ul>
        </details>}
      </>}
      <ConfirmDialog open={confirm} onOpenChange={setConfirm} title={t('confirmTitle')}
        description={consoleMode ? t('confirmMulti') : t('confirmSingle')} confirmLabel={t('apply')}
        cancelLabel={t('cancel')} pending={action.isPending} notifyOnError={false}
        onConfirm={async () => {
          if (!operation || operation.phase !== 'prepared') return
          try {
            await action.mutateAsync({ name: 'apply', operationId: operation.id })
          } finally {
            setConfirm(false)
          }
        }} />
    </SectionCard>
  )
}
