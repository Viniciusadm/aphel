import { PaginationSummary } from '@/components/ui/pagination-summary'
import { ArchiveRestore, MoreHorizontal, Settings2, Trash2 } from 'lucide-react'
import { createElement, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { DialogForm } from '@/components/ui/dialog-form'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Pagination } from '@/components/ui/pagination'
import { SearchInput } from '@/components/ui/search-input'
import { formatDateTime, ListSkeleton, NativeSelect, recordTypeIcon, RequirePermission, SETTINGS_MANAGE, useRecordFormatting } from '@/sdk'
import {
  MAX_RETENTION_DAYS,
  MIN_RETENTION_DAYS,
  TRASH_PER_PAGE,
  useTrash,
  useTrashMutations,
  useTrashSettings,
  useTrashTypes,
  useUpdateTrashSettings,
  type TrashFilters,
  type TrashItem,
  type TrashSettings,
} from './api'
import { useTrashTypeLabel } from './labels'

const DAY_MS = 86_400_000

function daysLeft(expiresAt: string | null | undefined): number | null {
  if (!expiresAt) return null
  return Math.max(0, Math.ceil((new Date(expiresAt).getTime() - Date.now()) / DAY_MS))
}

export function TrashPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <TrashSection />
    </RequirePermission>
  )
}

function TrashSection() {
  const { t } = useTranslation('trash')
  const [searchParams, setSearchParams] = useSearchParams()
  const typeParam = searchParams.get('type')
  const search = searchParams.get('search') ?? ''
  const types = useTrashTypes()
  const typeLabel = useTrashTypeLabel()
  const settings = useTrashSettings()
  const typeKeys = (types.data ?? []).map((type) => type.record_type)
  const isTrashType = (value: string | null): value is string => value !== null && typeKeys.includes(value)
  const filters: TrashFilters = {
    recordType: isTrashType(typeParam) ? typeParam : undefined,
    search: search || undefined,
  }
  const page = Math.max(1, Number(searchParams.get('page')) || 1)
  const trash = useTrash(filters, page)
  const [purging, setPurging] = useState<TrashItem | null>(null)
  const [configuring, setConfiguring] = useState(false)
  const { restore, purge } = useTrashMutations()

  const apply = (next: TrashFilters, nextPage = 1) => {
    const params = new URLSearchParams()
    if (next.recordType) params.set('type', next.recordType)
    if (next.search) params.set('search', next.search)
    if (nextPage > 1) params.set('page', String(nextPage))
    setSearchParams(params, { replace: true })
  }

  const restoreItem = (item: TrashItem) =>
    restore.mutate(item, {
      onSuccess: () => notifySuccess(t('restored', { name: item.title })),
      onError: (error) => notifyError(error),
    })

  const purgeItem = async () => {
    if (!purging) return
    await purge.mutateAsync(purging)
    notifySuccess(t('purged', { name: purging.title }))
  }

  const filtered = Boolean(filters.recordType || filters.search)

  return (
    <Page>
      <PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <Button variant="outline" onClick={() => setConfiguring(true)} disabled={!settings.data}>
            <Settings2 aria-hidden="true" />
            {settings.data ? t('retention.button', { count: settings.data.retention_days }) : t('retention.title')}
          </Button>
        }
      />
      <div className="flex flex-wrap items-center gap-3">
        <SearchInput
          wrapperClassName="w-full sm:w-72"
          value={search}
          placeholder={t('search')}
          aria-label={t('search')}
          onValueChange={(value) => apply({ ...filters, search: value.trim() || undefined })}
        />
        <NativeSelect
          aria-label={t('typeLabel')}
          className="w-full sm:w-56"
          value={filters.recordType ?? ''}
          onChange={(event) => apply({ ...filters, recordType: isTrashType(event.target.value) ? event.target.value : undefined })}
        >
          <option value="">{t('allTypes')}</option>
          {typeKeys.map((type) => (
            <option key={type} value={type}>
              {typeLabel(type, true)}
            </option>
          ))}
        </NativeSelect>
        {trash.data && trash.data.data.length > 0 && (
          <div className="ml-auto">
            <PaginationSummary page={page} perPage={TRASH_PER_PAGE} total={trash.data.meta.total} />
          </div>
        )}
      </div>
      {trash.isPending ? (
        <ListSkeleton />
      ) : trash.isError ? (
        <ErrorState onRetry={() => void trash.refetch()} />
      ) : trash.data.data.length === 0 ? (
        <EmptyState
          icon={Trash2}
          title={filtered ? t('emptyFiltered') : t('emptyTitle')}
          description={filtered ? t('emptyFilteredDescription') : t('emptyDescription')}
          action={
            filtered ? (
              <Button size="sm" variant="outline" onClick={() => setSearchParams({}, { replace: true })}>
                {t('clearFilters')}
              </Button>
            ) : undefined
          }
        />
      ) : (
        <>
          <ul className="divide-y rounded-lg border bg-card" aria-label={t('title')}>
            {trash.data.data.map((item) => (
              <TrashRow
                key={`${item.record_type}:${item.id}`}
                item={item}
                busy={restore.isPending || purge.isPending}
                onRestore={restoreItem}
                onPurge={setPurging}
              />
            ))}
          </ul>
          <Pagination page={page} perPage={TRASH_PER_PAGE} total={trash.data.meta.total} onPageChange={(next) => apply(filters, next)} />
        </>
      )}
      <ConfirmDialog
        open={purging !== null}
        onOpenChange={(open) => {
          if (!open) setPurging(null)
        }}
        variant="destructive"
        title={t('purgeTitle', { name: purging?.title ?? '' })}
        description={t('purgeDescription')}
        confirmLabel={t('purge')}
        confirmationText={purging?.title || undefined}
        pending={purge.isPending}
        onConfirm={purgeItem}
      />
      {settings.data && (
        <RetentionDialog open={configuring} onOpenChange={setConfiguring} current={settings.data} />
      )}
    </Page>
  )
}

interface TrashRowProps {
  item: TrashItem
  busy: boolean
  onRestore: (item: TrashItem) => void
  onPurge: (item: TrashItem) => void
}

function TrashRow({ item, busy, onRestore, onPurge }: TrashRowProps) {
  const { t } = useTranslation('trash')
  const formatting = useRecordFormatting()
  const typeLabel = useTrashTypeLabel()
  const left = daysLeft(item.expires_at)
  const urgent = left !== null && left <= 3
  return (
    <li className="flex flex-wrap items-center gap-3 p-4">
      <span className="inline-flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
        {createElement(recordTypeIcon(item.record_type), { className: 'size-5', 'aria-hidden': true })}
      </span>
      <div className="grid min-w-0 flex-1 gap-0.5">
        <span className="break-words font-medium">{item.title}</span>
        <span className="break-words text-sm text-muted-foreground">
          {[typeLabel(item.record_type), item.subtitle].filter(Boolean).join(' · ')}
        </span>
        <span className="text-xs text-muted-foreground">
          {t('deletedAt', { date: formatDateTime(item.deleted_at, formatting) })}
          {left !== null && (
            <>
              {' · '}
              <span className={urgent ? 'font-medium text-destructive' : undefined}>
                {left === 0 ? t('expiresToday') : t('expiresIn', { count: left })}
              </span>
            </>
          )}
        </span>
      </div>
      <div className="flex items-center gap-1">
        <Button size="sm" variant="outline" disabled={busy} aria-label={`${t('restore')}: ${item.title}`} onClick={() => onRestore(item)}>
          <ArchiveRestore aria-hidden="true" />
          {t('restore')}
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button size="icon" variant="ghost" className="size-8" aria-label={t('moreActions', { name: item.title })}>
              <MoreHorizontal aria-hidden="true" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem variant="destructive" disabled={busy} onSelect={() => onPurge(item)}>
              <Trash2 aria-hidden="true" />
              {t('purge')}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </li>
  )
}

function RetentionDialog({ open, onOpenChange, current }: { open: boolean; onOpenChange: (open: boolean) => void; current: TrashSettings }) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>{open && <RetentionForm key={current.retention_days} current={current} onClose={() => onOpenChange(false)} />}</DialogContent>
    </Dialog>
  )
}

function RetentionForm({ current, onClose }: { current: TrashSettings; onClose: () => void }) {
  const { t } = useTranslation('trash')
  const update = useUpdateTrashSettings()
  const [value, setValue] = useState(String(current.retention_days))
  const days = Number(value)
  const valid = Number.isInteger(days) && days >= MIN_RETENTION_DAYS && days <= MAX_RETENTION_DAYS

  return (
    <DialogForm
      title={t('retention.title')}
      description={t('retention.description')}
      submitLabel={t('retention.save')}
      cancelLabel={t('retention.cancel')}
      pending={update.isPending}
      submitDisabled={!valid || days === current.retention_days}
      onCancel={onClose}
      noValidate
      onSubmit={() =>
        update.mutate(
          { retention_days: days },
          {
            onSuccess: () => {
              notifySuccess(t('retention.saved'))
              onClose()
            },
            onError: (error) => notifyError(error),
          },
        )
      }
    >
      <Field label={t('retention.label')} description={valid ? t('retention.hint') : undefined} error={valid ? undefined : t('retention.invalid')}>
        {(control) => (
          <Input
            {...control}
            inputMode="numeric"
            className="w-32"
            placeholder={t('placeholders.retention')}
            value={value}
            onChange={(event) => setValue(event.target.value.replace(/\D/g, '').slice(0, 3))}
          />
        )}
      </Field>
    </DialogForm>
  )
}
