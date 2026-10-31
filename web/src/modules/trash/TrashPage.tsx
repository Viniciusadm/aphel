import { ArchiveRestore, Search, Trash2 } from 'lucide-react'
import { useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { Pagination } from '@/components/ui/pagination'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
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
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Page } from '@/components/ui/page'

export function TrashPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <div className="grid gap-6">
        <TrashSection />
        <RetentionCard />
      </div>
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
  const typeKeys = (types.data ?? []).map((type) => type.record_type)
  const isTrashType = (value: string | null): value is string => value !== null && typeKeys.includes(value)
  const filters: TrashFilters = {
    recordType: isTrashType(typeParam) ? typeParam : undefined,
    search: search || undefined,
  }
  const page = Math.max(1, Number(searchParams.get('page')) || 1)
  const trash = useTrash(filters, page)
  const [term, setTerm] = useState(search)
  const [purging, setPurging] = useState<TrashItem | null>(null)
  const { restore, purge } = useTrashMutations()

  const apply = (next: TrashFilters, nextPage = 1) => {
    const params = new URLSearchParams()
    if (next.recordType) params.set('type', next.recordType)
    if (next.search) params.set('search', next.search)
    if (nextPage > 1) params.set('page', String(nextPage))
    setSearchParams(params)
  }

  const submitSearch = (event: FormEvent) => {
    event.preventDefault()
    apply({ ...filters, search: term.trim() || undefined })
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
      <PageHeader title={t('title')} description={t('description')} />
      <div className="flex flex-wrap items-end gap-3">
        <label className="grid gap-1 text-sm">
          <span className="text-muted-foreground">{t('typeLabel')}</span>
          <NativeSelect
            value={filters.recordType ?? ''}
            onChange={(event) =>
              apply({ ...filters, recordType: isTrashType(event.target.value) ? event.target.value : undefined })
            }
          >
            <option value="">{t('allTypes')}</option>
            {typeKeys.map((type) => (
              <option key={type} value={type}>
                {typeLabel(type, true)}
              </option>
            ))}
          </NativeSelect>
        </label>
        <form role="search" className="flex min-w-0 flex-1 gap-2 sm:max-w-sm" onSubmit={submitSearch}>
          <Input
            aria-label={t('search')}
            placeholder={t('search')}
            value={term}
            onChange={(event) => setTerm(event.target.value)}
          />
          <Button type="submit" size="icon" variant="outline" aria-label={t('search')}>
            <Search aria-hidden="true" />
          </Button>
        </form>
      </div>
      {trash.isPending ? (
        <ListSkeleton />
      ) : trash.isError ? (
        <ErrorState onRetry={() => void trash.refetch()} />
      ) : trash.data.data.length === 0 ? (
        <EmptyState
          icon={Trash2}
          title={filtered ? t('emptyFiltered') : t('emptyTitle')}
          description={filtered ? undefined : t('emptyDescription')}
        />
      ) : (
        <>
          <TrashTable
            items={trash.data.data}
            busy={restore.isPending || purge.isPending}
            onRestore={restoreItem}
            onPurge={setPurging}
          />
          <Pagination
            page={page}
            perPage={TRASH_PER_PAGE}
            total={trash.data.meta.total}
            onPageChange={(next) => apply(filters, next)}
          />
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
    </Page>
  )
}

interface TrashTableProps {
  items: TrashItem[]
  busy: boolean
  onRestore: (item: TrashItem) => void
  onPurge: (item: TrashItem) => void
}

function TrashTable({ items, busy, onRestore, onPurge }: TrashTableProps) {
  const { t } = useTranslation('trash')
  const formatting = useRecordFormatting()
  const typeLabel = useTrashTypeLabel()
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead scope="col">
            {t('columns.record')}
          </TableHead>
          <TableHead scope="col" className="hidden sm:table-cell">
            {t('columns.type')}
          </TableHead>
          <TableHead scope="col" className="hidden md:table-cell">
            {t('columns.deletedAt')}
          </TableHead>
          <TableHead scope="col" className="hidden md:table-cell">
            {t('columns.expiresAt')}
          </TableHead>
          <TableHead scope="col" className="text-right">
            {t('columns.actions')}
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {items.map((item) => (
          <TableRow key={`${item.record_type}:${item.id}`} className="align-top">
            <TableCell>
              <div className="grid min-w-0 gap-0.5">
                <span className="font-medium break-words">{item.title}</span>
                {item.subtitle && <span className="text-xs break-words text-muted-foreground">{item.subtitle}</span>}
                <span className="text-xs text-muted-foreground sm:hidden">
                  {typeLabel(item.record_type)}
                </span>
              </div>
            </TableCell>
            <TableCell className="hidden sm:table-cell">
              {typeLabel(item.record_type)}
            </TableCell>
            <TableCell className="hidden whitespace-nowrap md:table-cell">
              {formatDateTime(item.deleted_at, formatting)}
            </TableCell>
            <TableCell className="hidden whitespace-nowrap md:table-cell">
              {item.expires_at ? formatDateTime(item.expires_at, formatting) : '—'}
            </TableCell>
            <TableCell>
              <div className="flex flex-wrap justify-end gap-1">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={busy}
                  aria-label={`${t('restore')}: ${item.title}`}
                  onClick={() => onRestore(item)}
                >
                  <ArchiveRestore aria-hidden="true" />
                  <span className="hidden sm:inline">{t('restore')}</span>
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  className="text-destructive"
                  disabled={busy}
                  aria-label={`${t('purge')}: ${item.title}`}
                  onClick={() => onPurge(item)}
                >
                  <Trash2 aria-hidden="true" />
                  <span className="hidden sm:inline">{t('purge')}</span>
                </Button>
              </div>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}

function RetentionCard() {
  const { t } = useTranslation('trash')
  const settings = useTrashSettings()
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('retention.title')}</CardTitle>
        <CardDescription>{t('retention.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        {settings.data ? (
          <RetentionForm key={settings.data.retention_days} current={settings.data} />
        ) : settings.isError ? (
          <ErrorState onRetry={() => void settings.refetch()} />
        ) : (
          <Skeleton className="h-10 w-48" />
        )}
      </CardContent>
    </Card>
  )
}

function RetentionForm({ current }: { current: TrashSettings }) {
  const { t } = useTranslation('trash')
  const update = useUpdateTrashSettings()
  const [value, setValue] = useState(String(current.retention_days))
  const days = Number(value)
  const valid = Number.isInteger(days) && days >= MIN_RETENTION_DAYS && days <= MAX_RETENTION_DAYS
  const hintId = 'trash-retention-hint'

  return (
    <form
      className="flex flex-wrap items-end gap-3"
      onSubmit={(event) => {
        event.preventDefault()
        if (!valid) return
        update.mutate(
          { retention_days: days },
          {
            onSuccess: () => notifySuccess(t('retention.saved')),
            onError: (error) => notifyError(error),
          },
        )
      }}
    >
      <div className="grid gap-1">
        <Label htmlFor="trash-retention">{t('retention.label')}</Label>
        <Input placeholder={t('placeholders.retention')}
          id="trash-retention"
          type="number"
          inputMode="numeric"
          min={MIN_RETENTION_DAYS}
          max={MAX_RETENTION_DAYS}
          className="w-32"
          value={value}
          aria-invalid={!valid}
          aria-describedby={hintId}
          onChange={(event) => setValue(event.target.value)}
        />
        <p id={hintId} className={valid ? 'text-xs text-muted-foreground' : 'text-xs text-destructive'}>
          {valid ? t('retention.hint') : t('retention.invalid')}
        </p>
      </div>
      <Button type="submit" size="sm" disabled={!valid || days === current.retention_days || update.isPending}>
        {t('retention.save')}
      </Button>
    </form>
  )
}
