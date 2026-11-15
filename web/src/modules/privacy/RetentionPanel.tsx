import { PaginationSummary } from '@/components/ui/pagination-summary'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { History, MoreHorizontal, Timer } from 'lucide-react'
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
import { formatDateTime, ListSkeleton, useRecordFormatting } from '@/sdk'
import { Pagination } from '@/components/ui/pagination'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
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

interface RetentionPanelProps {
  creating: boolean
  onCreatingChange: (creating: boolean) => void
}

export function RetentionPanel({ creating, onCreatingChange }: RetentionPanelProps) {
  const { t } = useTranslation('privacy')
  const policies = useRetentionPolicies()
  const mutations = usePolicyMutations()
  const notifyFailure = usePrivacyErrors()
  const [editing, setEditing] = useState<RetentionPolicy | null>(null)
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
        <div className="grid gap-1">
          <h3 className="font-medium">{t('retention.title')}</h3>
          <p className="text-sm text-muted-foreground">{t('retention.description')}</p>
        </div>
        {policies.isPending ? (
          <ListSkeleton rows={3} />
        ) : policies.isError ? (
          <ErrorState onRetry={() => void policies.refetch()} />
        ) : policies.data.length === 0 ? (
          <EmptyState icon={Timer} title={t('retention.empty')} description={t('retention.emptyDescription')} />
        ) : (
          <Table>
            <caption className="sr-only">{t('retention.listLabel')}</caption>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">{t('retention.columns.name')}</TableHead>
                <TableHead scope="col" className="hidden md:table-cell">{t('retention.columns.rule')}</TableHead>
                <TableHead scope="col" className="hidden lg:table-cell">{t('retention.columns.lastRun')}</TableHead>
                <TableHead scope="col" className="text-right">
                  <span className="sr-only">{t('retention.columns.actions')}</span>
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
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
            </TableBody>
          </Table>
        )}
      </section>

      <RunsHistory />

      {(creating || editing) && (
        <PolicyFormDialog
          policy={editing}
          onClose={() => {
            setEditing(null)
            onCreatingChange(false)
          }}
        />
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
  const { t, i18n } = useTranslation('privacy')
  const formatting = useRecordFormatting()
  const catalog = usePrivacyCatalog()
  const target = policy.record_type
    ? localizedLabel(
        catalog.data?.entities.find((entity) => entity.record_type === policy.record_type)?.labels ?? {},
        i18n.language,
        policy.record_type,
      ).toLocaleLowerCase(i18n.language)
    : null
  return (
    <TableRow>
      <TableCell className="max-w-64">
        <span className="block truncate font-medium">{policy.name}</span>
        <Badge variant={policy.is_enabled ? 'success' : 'outline'}>
          {policy.is_enabled ? t('retention.active') : t('retention.inactive')}
        </Badge>
      </TableCell>
      <TableCell className="hidden text-muted-foreground md:table-cell">
        {t(`actionSummaries.${policy.action as 'archive_inactive'}`, { target, days: policy.threshold_days })}
      </TableCell>
      <TableCell className="hidden text-muted-foreground lg:table-cell">
        {policy.last_run_at ? formatDateTime(policy.last_run_at, formatting) : t('retention.neverRun')}
        {policy.previewed_count !== null && policy.previewed_count !== undefined && policy.previewed_at && (
          <span className="block text-xs">
            {t('retention.lastPreview', { count: policy.previewed_count, date: formatDateTime(policy.previewed_at, formatting) })}
          </span>
        )}
      </TableCell>
      <TableCell className="text-right">
        <div className="flex items-center justify-end gap-1">
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
              <DropdownMenuItem disabled={busy} onSelect={onPreview}>
                {t('actions.preview')}
              </DropdownMenuItem>
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
      </TableCell>
    </TableRow>
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
          <Table>
            <caption className="sr-only">{t('runs.title')}</caption>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">{t('runs.columns.policy')}</TableHead>
                <TableHead scope="col">{t('runs.columns.status')}</TableHead>
                <TableHead scope="col" className="hidden md:table-cell">{t('runs.columns.result')}</TableHead>
                <TableHead scope="col" className="hidden sm:table-cell">{t('runs.columns.when')}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {runs.data.data.map((run) => (
                <TableRow key={run.id}>
                  <TableCell className="max-w-56">
                    <span className="block truncate font-medium">{run.policy_name}</span>
                    <Badge variant="outline">{t(`runs.triggers.${run.trigger as 'schedule'}`)}</Badge>
                  </TableCell>
                  <TableCell>
                    <StatusBadge status={run.status} />
                    {run.error_message && (
                      <SharedFieldError>{run.status === 'failed' ? t('runs.failed') : t('runs.partial')}</SharedFieldError>
                    )}
                  </TableCell>
                  <TableCell className="hidden text-muted-foreground md:table-cell">
                    {t('runs.summary', { affected: run.affected, files: run.files_removed, errors: run.errors })}
                  </TableCell>
                  <TableCell className="hidden text-muted-foreground sm:table-cell">
                    {formatDateTime(run.started_at ?? run.created_at, formatting)}
                    {run.requested_by && <span className="block text-xs">{run.requested_by.display_name}</span>}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          <Pagination page={page} perPage={RUNS_PER_PAGE} total={runs.data.meta.total} onPageChange={setPage} />
        </>
      )}
    </section>
  )
}
