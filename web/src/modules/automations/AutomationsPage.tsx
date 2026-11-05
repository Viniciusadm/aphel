import { useMutation } from '@tanstack/react-query'
import { Clock, Pause, Play, Plus, Workflow } from 'lucide-react'
import { useEffect, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, NativeSelect, useCustomObjects, useRecordFormatting } from '@/sdk'
import { AUTOMATIONS_PER_PAGE, automationPath, automationsApi, useAutomationCache, useAutomationCatalog, useAutomations } from './api'
import { Pager } from './components/Pager'
import { AutomationStatusBadge } from './components/StatusBadges'
import { CreateAutomationDialog } from './CreateAutomationDialog'
import { automatableRecordTypes, useAutomationLabels } from './lib/labels'
import { AUTOMATION_STATUSES, type Automation, type AutomationStatus } from './types'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'
import { Field } from '@/components/ui/field'
import { SearchInput } from '@/components/ui/search-input'

export const AUTOMATIONS_PERMISSION = 'automations.manage'

export function AutomationsPage() {
  const { t } = useTranslation('automations')
  const { hasPermission } = useSession()
  if (!hasPermission(AUTOMATIONS_PERMISSION)) {
    return <EmptyState title={t('page.noAccessTitle')} description={t('page.noAccessDescription')} />
  }
  return <AutomationsList />
}

function useDebounced<T>(value: T, delay = 300): T {
  const [debounced, setDebounced] = useState(value)
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), delay)
    return () => clearTimeout(timer)
  }, [value, delay])
  return debounced
}

function AutomationsList() {
  const { t } = useTranslation('automations')
  const searchId = useId()
  const catalog = useAutomationCatalog()
  const objects = useCustomObjects()
  const labels = useAutomationLabels(catalog.data)
  const [search, setSearch] = useState('')
  const [status, setStatus] = useState<AutomationStatus | ''>('')
  const [recordType, setRecordType] = useState('')
  const [page, setPage] = useState(1)
  const [creating, setCreating] = useState(false)
  const term = useDebounced(search.trim())
  const automations = useAutomations({
    search: term || undefined,
    status: status || undefined,
    record_type: recordType || undefined,
    page,
  })
  const recordTypes = automatableRecordTypes(
    catalog.data,
    (objects.data ?? []).filter((object) => object.is_active).map((object) => object.record_type),
  )

  let body
  if (automations.isPending) {
    body = (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-20 w-full" />
        <Skeleton className="h-20 w-full" />
        <Skeleton className="h-20 w-full" />
      </div>
    )
  } else if (automations.isError) {
    body = <ErrorState onRetry={() => void automations.refetch()} />
  } else if (automations.data.data.length === 0) {
    const filtered = Boolean(term || status || recordType)
    body = (
      <EmptyState
        icon={Workflow}
        title={filtered ? t('page.noResultsTitle') : t('page.emptyTitle')}
        description={filtered ? t('page.noResultsDescription') : t('page.emptyDescription')}
        action={
          filtered ? undefined : (
            <Button size="sm" onClick={() => setCreating(true)} disabled={!catalog.data}>
              <Plus aria-hidden="true" />
              {t('page.new')}
            </Button>
          )
        }
      />
    )
  } else {
    body = (
      <>
        <ul className="grid gap-2" aria-label={t('page.listLabel')}>
          {automations.data.data.map((automation) => (
            <AutomationRow key={automation.id} automation={automation} labels={labels} />
          ))}
        </ul>
        <Pager page={page} perPage={AUTOMATIONS_PER_PAGE} total={automations.data.meta.total} onPageChange={setPage} />
      </>
    )
  }

  return (
    <Page>
<PageHeader title={t('page.title')} description={t('page.description')} actions={<>
        <Button onClick={() => setCreating(true)} disabled={!catalog.data}>
          <Plus aria-hidden="true" />
          {t('page.new')}
        </Button>
      
      </>} />
      <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_12rem_12rem]">
        <Field id={searchId} label={t('page.search')} hideLabel>
          <SearchInput
              id={searchId}
              value={search}
              placeholder={t('page.search')}
              onValueChange={(value) => {
                setSearch(value)
                setPage(1)
              }}
              
            />
        </Field>
        <Field label={t('page.status')} hideLabel>
          {(fieldControl) => <NativeSelect
            {...fieldControl}
            value={status}
            onChange={(event) => {
              setStatus(event.target.value as AutomationStatus | '')
              setPage(1)
            }}
            className="w-full"
          >
            <option value="">{t('page.allStatuses')}</option>
            {AUTOMATION_STATUSES.map((item) => (
              <option key={item} value={item}>
                {t(`status.${item}`)}
              </option>
            ))}
          </NativeSelect>}
        </Field>
        <Field label={t('page.recordType')} hideLabel>
          {(fieldControl) => <NativeSelect
            {...fieldControl}
            value={recordType}
            onChange={(event) => {
              setRecordType(event.target.value)
              setPage(1)
            }}
            className="w-full"
          >
            <option value="">{t('page.allRecordTypes')}</option>
            {recordTypes.map((item) => (
              <option key={item} value={item}>
                {labels.recordTypeLabel(item)}
              </option>
            ))}
          </NativeSelect>}
        </Field>
      </div>
      {body}
      {catalog.data && (
        <CreateAutomationDialog open={creating} onOpenChange={setCreating} catalog={catalog.data} recordTypes={recordTypes} />
      )}
    </Page>
  )
}

function AutomationRow({ automation, labels }: { automation: Automation; labels: ReturnType<typeof useAutomationLabels> }) {
  const { t } = useTranslation('automations')
  const formatting = useRecordFormatting()
  const cache = useAutomationCache()
  const stats = automation.stats
  const errorRate = stats && stats.runs_30d > 0 ? Math.round((stats.failed_30d / stats.runs_30d) * 100) : null

  const toggle = useMutation({
    mutationFn: () => (automation.status === 'active' ? automationsApi.pause(automation.id) : automationsApi.activate(automation.id)),
    onSuccess: async (updated) => {
      await cache.store(updated)
      notifySuccess(updated.status === 'active' ? t('toast.activated') : t('toast.paused'))
    },
    onError: (error) => notifyError(error),
  })

  return (
    <li className="grid gap-3 rounded-lg border bg-card p-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center">
      <div className="grid min-w-0 gap-1">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <Link to={automationPath(automation.id)} className="truncate font-medium underline-offset-4 hover:underline">
            {automation.name}
          </Link>
          <AutomationStatusBadge status={automation.status} />
          {automation.version > 0 && <span className="text-xs text-muted-foreground">{t('page.version', { version: automation.version })}</span>}
        </div>
        <p className="truncate text-sm text-muted-foreground">
          {labels.recordTypeLabel(automation.record_type)} · {labels.triggerLabel(automation.trigger.type)}
        </p>
        <dl className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
          <div className="flex gap-1">
            <dt>{t('page.lastRun')}</dt>
            <dd className="text-foreground">
              {stats?.last_run_at ? formatDateTime(stats.last_run_at, formatting) : t('page.never')}
            </dd>
          </div>
          <div className="flex gap-1">
            <dt>{t('page.runs30d')}</dt>
            <dd className="text-foreground tabular-nums">{stats ? stats.runs_30d : '—'}</dd>
          </div>
          <div className="flex gap-1">
            <dt>{t('page.errorRate')}</dt>
            <dd className={errorRate && errorRate > 0 ? 'text-destructive tabular-nums' : 'text-foreground tabular-nums'}>
              {errorRate === null ? '—' : `${errorRate}%`}
            </dd>
          </div>
          {stats && stats.waiting > 0 && (
            <div className="flex items-center gap-1">
              <Clock className="size-3" aria-hidden="true" />
              <dt className="sr-only">{t('page.waiting')}</dt>
              <dd className="text-foreground">{t('page.waitingCount', { count: stats.waiting })}</dd>
            </div>
          )}
        </dl>
      </div>
      <div className="flex gap-2 sm:justify-end">
        {automation.version > 0 && (
          <Button type="button" variant="outline" size="sm" disabled={toggle.isPending} onClick={() => toggle.mutate()}>
            {automation.status === 'active' ? <Pause aria-hidden="true" /> : <Play aria-hidden="true" />}
            {automation.status === 'active' ? t('actionsBar.pause') : t('actionsBar.activate')}
          </Button>
        )}
        <Button asChild size="sm" variant="secondary">
          <Link to={automationPath(automation.id)}>{t('page.open')}</Link>
        </Button>
      </div>
    </li>
  )
}
