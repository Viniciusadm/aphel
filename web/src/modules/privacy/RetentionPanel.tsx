import { PaginationSummary } from '@/components/ui/pagination-summary'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { History, MoreHorizontal, Plus, Timer } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { formatDate, formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { Pagination } from '@/components/ui/pagination'
import {
  RUNS_PER_PAGE,
  usePolicyMutations,
  usePrivacyCatalog,
  useRetentionPolicies,
  useRetentionRuns,
  type RetentionPolicy,
  type RetentionPreview,
} from './api'
import { localizedLabel, usePrivacyErrors } from './lib/shared'
import { PolicyFormDialog } from './PolicyFormDialog'
import { StatusBadge } from './StatusBadge'

type PendingAction = { kind: 'enable' | 'run'; policy: RetentionPolicy; preview: RetentionPreview }

function isDestructive(policy: RetentionPolicy): boolean {
  return policy.action !== 'archive_inactive'
}

export function RetentionPanel() {
  const { t } = useTranslation('privacy')
  const policies = useRetentionPolicies()
  const mutations = usePolicyMutations()
  const notifyFailure = usePrivacyErrors()
  const [editing, setEditing] = useState<RetentionPolicy | 'new' | null>(null)
  const [deleting, setDeleting] = useState<RetentionPolicy | null>(null)
  const [previewed, setPreviewed] = useState<{ policy: RetentionPolicy; preview: RetentionPreview } | null>(null)
  const [pending, setPending] = useState<PendingAction | null>(null)

  const preview = (policy: RetentionPolicy, next?: 'enable' | 'run') =>
    mutations.preview.mutate(policy.id, {
      onSuccess: (result) => {
        if (next) setPending({ kind: next, policy, preview: result })
        else setPreviewed({ policy, preview: result })
      },
      onError: notifyFailure,
    })

  const disable = (policy: RetentionPolicy) =>
    mutations.disable.mutate(policy.id, {
      onSuccess: () => notifySuccess(t('retention.disabled', { name: policy.name })),
      onError: notifyFailure,
    })

  const confirmPending = async () => {
    if (!pending) return
    try {
      if (pending.kind === 'enable') {
        await mutations.enable.mutateAsync(pending.policy.id)
        notifySuccess(t('retention.enabled', { name: pending.policy.name }))
      } else {
        await mutations.run.mutateAsync(pending.policy.id)
        notifySuccess(t('retention.runQueued', { name: pending.policy.name }))
      }
    } catch (error) {
      notifyFailure(error)
    }
  }

  const confirmDelete = async () => {
    if (!deleting) return
    try {
      await mutations.remove.mutateAsync(deleting.id)
      notifySuccess(t('retention.deleted', { name: deleting.name }))
    } catch (error) {
      notifyFailure(error)
    }
  }

  return (
    <div className="grid min-w-0 gap-6">
      <section className="grid min-w-0 gap-3">
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div className="grid gap-1">
            <h3 className="font-medium">{t('retention.title')}</h3>
            <p className="text-sm text-muted-foreground">{t('retention.description')}</p>
          </div>
          <Button onClick={() => setEditing('new')}>
            <Plus aria-hidden="true" />
            {t('retention.create')}
          </Button>
        </div>
        {policies.isPending ? (
          <ListSkeleton rows={3} />
        ) : policies.isError ? (
          <ErrorState onRetry={() => void policies.refetch()} />
        ) : policies.data.length === 0 ? (
          <EmptyState icon={Timer} title={t('retention.empty')} description={t('retention.emptyDescription')} />
        ) : (
          <ul className="grid gap-2">
            {policies.data.map((policy) => (
              <PolicyRow
                key={policy.id}
                policy={policy}
                busy={mutations.preview.isPending}
                onPreview={() => preview(policy)}
                onEnable={() => preview(policy, 'enable')}
                onDisable={() => disable(policy)}
                onRun={() => preview(policy, 'run')}
                onEdit={() => setEditing(policy)}
                onDelete={() => setDeleting(policy)}
              />
            ))}
          </ul>
        )}
      </section>

      <RunsHistory />

      {editing && (
        <PolicyFormDialog policy={editing === 'new' ? null : editing} onClose={() => setEditing(null)} />
      )}

      <ConfirmDialog
        open={previewed !== null}
        onOpenChange={(open) => !open && setPreviewed(null)}
        title={t('retention.previewTitle')}
        description={
          previewed
            ? t(`retention.previewResult.${previewed.policy.action as 'archive_inactive'}`, {
                count: previewed.preview.affected,
              })
            : undefined
        }
        confirmLabel={t('actions.close')}
        onConfirm={() => setPreviewed(null)}
      />

      <ConfirmDialog
        open={pending !== null}
        onOpenChange={(open) => !open && setPending(null)}
        title={pending ? t(pending.kind === 'enable' ? 'retention.enableTitle' : 'retention.runTitle') : ''}
        description={
          pending
            ? `${t(`retention.previewResult.${pending.policy.action as 'archive_inactive'}`, {
                count: pending.preview.affected,
              })} ${t(pending.kind === 'enable' ? 'retention.enableNotice' : 'retention.runNotice')}`
            : undefined
        }
        confirmLabel={pending ? t(pending.kind === 'enable' ? 'actions.enable' : 'actions.runNow') : undefined}
        variant={pending && isDestructive(pending.policy) ? 'destructive' : 'default'}
        confirmationText={pending && isDestructive(pending.policy) ? pending.policy.name : undefined}
        notifyOnError={false}
        onConfirm={confirmPending}
      />

      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={t('retention.deleteTitle')}
        description={deleting ? t('retention.deleteDescription', { name: deleting.name }) : undefined}
        variant="destructive"
        notifyOnError={false}
        onConfirm={confirmDelete}
      />
    </div>
  )
}

interface PolicyRowProps {
  policy: RetentionPolicy
  busy: boolean
  onPreview: () => void
  onEnable: () => void
  onDisable: () => void
  onRun: () => void
  onEdit: () => void
  onDelete: () => void
}

function PolicyRow({ policy, busy, onPreview, onEnable, onDisable, onRun, onEdit, onDelete }: PolicyRowProps) {
  const { t } = useTranslation('privacy')
  const formatting = useRecordFormatting()
  const catalog = usePrivacyCatalog()
  const { i18n } = useTranslation()
  const target = policy.record_type ? localizedLabel(catalog.data?.entities.find((entity) => entity.record_type === policy.record_type)?.labels ?? {}, i18n.language, policy.record_type) : null
  return (
    <li className="flex min-w-0 flex-col gap-2 rounded-lg border bg-card p-3 sm:flex-row sm:items-center sm:justify-between">
      <div className="grid min-w-0 gap-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-medium break-words">{policy.name}</span>
          <Badge variant={policy.is_enabled ? 'success' : 'outline'}>
            {policy.is_enabled ? t('retention.active') : t('retention.inactive')}
          </Badge>
        </div>
        <p className="text-sm text-muted-foreground">
          {t(`actionSummaries.${policy.action as 'archive_inactive'}`, { target, days: policy.threshold_days })}
        </p>
        <p className="text-xs text-muted-foreground">
          {policy.last_run_at
            ? t('retention.lastRun', { date: formatDateTime(policy.last_run_at, formatting) })
            : t('retention.neverRun')}
          {policy.previewed_count !== null && policy.previewed_count !== undefined && policy.previewed_at
            ? ` · ${t('retention.lastPreview', { count: policy.previewed_count, date: formatDate(policy.previewed_at, formatting) })}`
            : ''}
        </p>
      </div>
      <div className="flex shrink-0 flex-wrap items-center gap-2">
        <Button variant="outline" size="sm" disabled={busy} onClick={onPreview}>
          {t('actions.preview')}
        </Button>
        {policy.is_enabled ? (
          <Button variant="outline" size="sm" onClick={onDisable}>
            {t('actions.disable')}
          </Button>
        ) : (
          <Button size="sm" disabled={busy} onClick={onEnable}>
            {t('actions.enable')}
          </Button>
        )}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon" aria-label={t('actions.moreFor', { name: policy.name })}>
              <MoreHorizontal />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem disabled={busy} onSelect={onRun}>
              {t('actions.runNow')}
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={onEdit}>{t('actions.edit')}</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive" onSelect={onDelete}>
              {t('actions.delete')}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </li>
  )
}

function RunsHistory() {
  const { t } = useTranslation('privacy')
  const formatting = useRecordFormatting()
  const [page, setPage] = useState(1)
  const runs = useRetentionRuns(undefined, page)

  return (
    <section className="grid min-w-0 gap-3">
      <div className="grid gap-1">
        <h3 className="font-medium">{t('runs.title')}</h3>
        <p className="text-sm text-muted-foreground">{t('runs.description')}</p>
      </div>
      {runs.isPending ? (
        <ListSkeleton rows={3} />
      ) : runs.isError ? (
        <ErrorState onRetry={() => void runs.refetch()} />
      ) : runs.data.data.length === 0 ? (
        <EmptyState icon={History} title={t('runs.empty')} />
      ) : (
        <>
          <PaginationSummary page={page} perPage={RUNS_PER_PAGE} total={runs.data.meta.total} />
          <ul className="grid gap-2">
            {runs.data.data.map((run) => (
              <li key={run.id} className="grid min-w-0 gap-1 rounded-lg border p-3 text-sm">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium break-words">{run.policy_name}</span>
                  <StatusBadge status={run.status} />
                  <Badge variant="outline">{t(`runs.triggers.${run.trigger as 'schedule'}`)}</Badge>
                </div>
                <p className="text-muted-foreground">
                  {t('runs.summary', { affected: run.affected, files: run.files_removed, errors: run.errors })}
                </p>
                <p className="text-xs text-muted-foreground">
                  {formatDateTime(run.started_at ?? run.created_at, formatting)}
                  {run.requested_by ? ` · ${run.requested_by.display_name}` : ''}
                </p>
                {run.error_message && (
                  <SharedFieldError>
                    {run.status === 'failed' ? t('runs.failed') : t('runs.partial')}
                  </SharedFieldError>
                )}
              </li>
            ))}
          </ul>
          <Pagination page={page} perPage={RUNS_PER_PAGE} total={runs.data.meta.total} onPageChange={setPage} />
        </>
      )}
    </section>
  )
}
