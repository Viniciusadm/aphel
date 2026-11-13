import { Layers, Pencil, Plus, Trash2, Users } from 'lucide-react'
import { type ReactNode, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { SegmentFilterSummary } from './SegmentFilterSummary'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import {SEGMENT_RECORD_TYPES, isSegmentRecordType, segmentReadPermission, segmentsApi, useInvalidateSegments, useSegments, type Segment, type SegmentRecordType} from '@/sdk'
import { SegmentDialog } from './SegmentDialog'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export const SEGMENT_TYPE_PARAM = 'type'

export function SegmentsPage() {
  const { t } = useTranslation('segments')
  const { hasPermission } = useSession()
  const [params, setParams] = useSearchParams()
  const readable = SEGMENT_RECORD_TYPES.filter((recordType) => hasPermission(segmentReadPermission(recordType)))
  const requested = params.get(SEGMENT_TYPE_PARAM)
  const recordType = isSegmentRecordType(requested) && readable.includes(requested) ? requested : readable[0]
  const canManage = hasPermission('settings.manage')
  const invalidate = useInvalidateSegments()
  const [editor, setEditor] = useState<{ segment: Segment | null } | null>(null)
  const [deleting, setDeleting] = useState<Segment | null>(null)

  const remove = async (segment: Segment) => {
    await segmentsApi.remove(segment.id)
    await invalidate()
    notifySuccess(t('toast.deleted', { name: segment.name }))
  }

  const newButton = canManage && recordType ? (
    <Button size="sm" onClick={() => setEditor({ segment: null })}>
      <Plus aria-hidden="true" />
      {t('page.new')}
    </Button>
  ) : undefined

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} actions={newButton} />
      {recordType ? (
        <>
          <div>
            <SegmentedControl
              aria-label={t('page.typesLabel')}
              value={recordType}
              onValueChange={(candidate) =>
                setParams((current) => {
                  const next = new URLSearchParams(current)
                  next.set(SEGMENT_TYPE_PARAM, candidate)
                  return next
                })
              }
              options={readable.map((candidate) => ({ value: candidate, label: t(`recordTypes.${candidate}`) }))}
            />
          </div>
          <SegmentList
            key={recordType}
            recordType={recordType}
            canManage={canManage}
            emptyAction={newButton}
            onEdit={(segment) => setEditor({ segment })}
            onDelete={setDeleting}
          />
          {canManage && (
            <SegmentDialog
              open={editor !== null}
              recordType={recordType}
              editing={editor?.segment ?? null}
              onOpenChange={(open) => !open && setEditor(null)}
              onSaved={() => void invalidate()}
            />
          )}
          <ConfirmDialog
            open={deleting !== null}
            onOpenChange={(open) => !open && setDeleting(null)}
            title={t('deleteDialog.title')}
            description={t('deleteDialog.description', { name: deleting?.name ?? '' })}
            confirmLabel={t('deleteDialog.confirm')}
            variant="destructive"
            onConfirm={() => (deleting ? remove(deleting) : undefined)}
          />
        </>
      ) : (
        <EmptyState title={t('page.noAccessTitle')} description={t('page.noAccessDescription')} />
      )}
    </Page>
  )
}

function SegmentList({
  recordType,
  canManage,
  emptyAction,
  onEdit,
  onDelete,
}: {
  recordType: SegmentRecordType
  canManage: boolean
  emptyAction: ReactNode
  onEdit: (segment: Segment) => void
  onDelete: (segment: Segment) => void
}) {
  const { t } = useTranslation('segments')
  const formatting = useRecordFormatting()
  const segments = useSegments(recordType)

  if (segments.isPending) {
    return (
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3" aria-busy="true">
        <Skeleton className="h-32 w-full" />
        <Skeleton className="h-32 w-full" />
        <Skeleton className="h-32 w-full" />
      </div>
    )
  }
  if (segments.isError) return <ErrorState onRetry={() => void segments.refetch()} />
  if (segments.data.length === 0) {
    return (
      <EmptyState
        icon={Layers}
        title={t(`page.empty.${recordType}.title`)}
        description={t(`page.empty.${recordType}.description`)}
        action={emptyAction}
      />
    )
  }
  return (
    <ul className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3" aria-label={t('page.listLabel')}>
      {segments.data.map((segment) => (
        <li key={segment.id} className="flex min-w-0 flex-col gap-3 rounded-xl border bg-card p-4">
          <div className="flex min-w-0 items-start justify-between gap-2">
            <Link
              to={`/segments/${encodeURIComponent(segment.id)}`}
              className="min-w-0 truncate text-base font-semibold underline-offset-4 hover:underline"
            >
              {segment.name}
            </Link>
            <span className="inline-flex shrink-0 items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-xs font-medium tabular-nums">
              <Users className="size-3" aria-hidden="true" />
              {segment.member_count === null || segment.member_count === undefined
                ? t('page.countUnavailable')
                : t('page.count', { count: segment.member_count })}
            </span>
          </div>
          {segment.description && <p className="line-clamp-2 text-sm text-muted-foreground">{segment.description}</p>}
          <SegmentFilterSummary recordType={recordType} filter={segment.filter} />
          <div className="mt-auto flex items-center justify-between gap-2 pt-1">
            <span className="text-xs text-muted-foreground">
              {t('page.updated', { date: formatDateTime(segment.updated_at, formatting) })}
            </span>
            {canManage && (
              <div className="flex gap-1">
                <Button size="icon" variant="ghost" className="size-8" aria-label={t('page.editLabel', { name: segment.name })} onClick={() => onEdit(segment)}>
                  <Pencil aria-hidden="true" />
                </Button>
                <Button size="icon" variant="ghost" className="size-8" aria-label={t('page.deleteLabel', { name: segment.name })} onClick={() => onDelete(segment)}>
                  <Trash2 aria-hidden="true" />
                </Button>
              </div>
            )}
          </div>
        </li>
      ))}
    </ul>
  )
}
