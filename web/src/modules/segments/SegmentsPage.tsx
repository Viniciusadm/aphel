import { Pencil, Plus, Trash2, Users } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import {
  SEGMENT_RECORD_TYPES,
  isSegmentRecordType,
  segmentReadPermission,
  segmentsApi,
  useInvalidateSegments,
  useSegments,
  type Segment,
  type SegmentRecordType,
} from './api'
import { SegmentDialog } from './SegmentDialog'

export const SEGMENT_TYPE_PARAM = 'type'

export function SegmentsPage() {
  const { t } = useTranslation('segments')
  const { hasPermission } = useSession()
  const [params, setParams] = useSearchParams()
  const readable = SEGMENT_RECORD_TYPES.filter((recordType) => hasPermission(segmentReadPermission(recordType)))
  const requested = params.get(SEGMENT_TYPE_PARAM)
  const recordType = isSegmentRecordType(requested) && readable.includes(requested) ? requested : readable[0]

  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-1">
        <h1 className="text-xl font-semibold">{t('page.title')}</h1>
        <p className="text-sm text-muted-foreground">{t('page.description')}</p>
      </div>
      {recordType ? (
        <>
          <nav aria-label={t('page.typesLabel')} className="flex gap-1 overflow-x-auto border-b">
            {readable.map((candidate) => (
              <button
                key={candidate}
                type="button"
                aria-current={candidate === recordType ? 'page' : undefined}
                onClick={() => setParams({ [SEGMENT_TYPE_PARAM]: candidate })}
                className={cn(
                  '-mb-px border-b-2 px-3 py-2 text-sm font-medium whitespace-nowrap transition-colors',
                  candidate === recordType
                    ? 'border-primary text-foreground'
                    : 'border-transparent text-muted-foreground hover:text-foreground',
                )}
              >
                {t(`recordTypes.${candidate}`)}
              </button>
            ))}
          </nav>
          <SegmentList key={recordType} recordType={recordType} />
        </>
      ) : (
        <EmptyState title={t('page.noAccessTitle')} description={t('page.noAccessDescription')} />
      )}
    </div>
  )
}

function SegmentList({ recordType }: { recordType: SegmentRecordType }) {
  const { t } = useTranslation('segments')
  const { hasPermission } = useSession()
  const canManage = hasPermission('settings.manage')
  const segments = useSegments(recordType)
  const invalidate = useInvalidateSegments()
  const [editor, setEditor] = useState<{ segment: Segment | null } | null>(null)
  const [deleting, setDeleting] = useState<Segment | null>(null)

  const remove = async (segment: Segment) => {
    await segmentsApi.remove(segment.id)
    await invalidate()
    notifySuccess(t('toast.deleted', { name: segment.name }))
  }

  const newButton = canManage ? (
    <Button size="sm" onClick={() => setEditor({ segment: null })}>
      <Plus aria-hidden="true" />
      {t('page.new')}
    </Button>
  ) : undefined

  let body
  if (segments.isPending) {
    body = (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-16 w-full" />
        <Skeleton className="h-16 w-full" />
      </div>
    )
  } else if (segments.isError) {
    body = <ErrorState onRetry={() => void segments.refetch()} />
  } else if (segments.data.length === 0) {
    body = <EmptyState title={t('page.emptyTitle')} description={t('page.emptyDescription')} action={newButton} />
  } else {
    body = (
      <ul className="grid gap-2 sm:grid-cols-2" aria-label={t('page.listLabel')}>
        {segments.data.map((segment) => (
          <li key={segment.id} className="flex min-w-0 flex-col gap-2 rounded-lg border bg-card p-4">
            <div className="flex min-w-0 items-start justify-between gap-2">
              <div className="grid min-w-0 gap-0.5">
                <Link
                  to={`/segments/${encodeURIComponent(segment.id)}`}
                  className="truncate font-medium underline-offset-4 hover:underline"
                >
                  {segment.name}
                </Link>
                {segment.description && (
                  <p className="line-clamp-2 text-sm text-muted-foreground">{segment.description}</p>
                )}
              </div>
              <span className="inline-flex shrink-0 items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-xs font-medium tabular-nums">
                <Users className="size-3" aria-hidden="true" />
                {segment.member_count === null || segment.member_count === undefined
                  ? t('page.countUnavailable')
                  : t('page.count', { count: segment.member_count })}
              </span>
            </div>
            {canManage && (
              <div className="flex justify-end gap-1">
                <Button
                  size="sm"
                  variant="ghost"
                  aria-label={t('page.editLabel', { name: segment.name })}
                  onClick={() => setEditor({ segment })}
                >
                  <Pencil aria-hidden="true" />
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  aria-label={t('page.deleteLabel', { name: segment.name })}
                  onClick={() => setDeleting(segment)}
                >
                  <Trash2 aria-hidden="true" />
                </Button>
              </div>
            )}
          </li>
        ))}
      </ul>
    )
  }

  return (
    <section className="flex flex-col gap-3">
      {segments.data && segments.data.length > 0 && newButton && <div className="flex justify-end">{newButton}</div>}
      {body}
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
    </section>
  )
}
