import { Link2, Lock, Unlink } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { notifySuccess } from '@/components/feedback'
import { RecordFieldInput } from '@/components/fields'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { getRecordResource } from '@/features/records/resources'
import {
  RELATED_RECORDS_PAGE_SIZE,
  useRecordRelations,
  useRelatedRecords,
  useRelationMutations,
  type RelationInfo,
} from './api'
import { notifyObjectsError } from './errors'

export interface RelationPanelProps {
  recordType: string
  recordId: string
  relation: RelationInfo
  readOnly?: boolean
}

export function RelationPanel({ recordType, recordId, relation, readOnly = false }: RelationPanelProps) {
  const { t } = useTranslation('objects')
  const pickerId = useId()
  const [page, setPage] = useState(1)
  const related = useRelatedRecords(recordType, recordId, relation.key, page)
  const { link, unlink } = useRelationMutations(recordType, recordId, relation)
  const targetResource = getRecordResource(relation.target_type)

  if (!relation.readable) {
    return <EmptyState icon={Lock} title={t('relations.noAccessTitle')} description={t('relations.noAccess')} />
  }

  const total = related.data?.meta.total ?? relation.total ?? 0
  const items = related.data?.data ?? []
  const pages = Math.max(1, Math.ceil(total / RELATED_RECORDS_PAGE_SIZE))
  const canLink = !readOnly && Boolean(targetResource) && !(relation.single && total > 0)
  const busy = link.isPending || unlink.isPending

  const linkRecord = (targetId: string | null) => {
    if (!targetId) return
    link.mutate(targetId, {
      onSuccess: (result) =>
        notifySuccess(result.linked ? t('relations.linked') : t('relations.alreadyLinked')),
      onError: notifyObjectsError,
    })
  }

  const unlinkRecord = (targetId: string) =>
    unlink.mutate(targetId, {
      onSuccess: () => notifySuccess(t('relations.unlinked')),
      onError: notifyObjectsError,
    })

  return (
    <div className="grid gap-3">
      {canLink && (
        <div className="grid gap-2.5">
          <Label htmlFor={pickerId}>{t('relations.link', { label: relation.label })}</Label>
          <RecordFieldInput
            id={pickerId}
            targetRecordType={relation.target_type}
            value={null}
            clearable={false}
            disabled={busy}
            onChange={linkRecord}
          />
        </div>
      )}
      {related.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-11 w-full" />
          <Skeleton className="h-11 w-full" />
        </div>
      ) : related.isError ? (
        <ErrorState title={t('relations.loadError')} onRetry={() => void related.refetch()} />
      ) : items.length === 0 ? (
        <EmptyState title={t('relations.empty', { label: relation.label })} />
      ) : (
        <ul aria-label={relation.label} className="grid gap-2">
          {items.map((item) => {
            const href = getRecordResource(item.record_type)?.detailPath?.(item.id)
            const title = item.title || item.id
            return (
              <li key={item.id} className="flex min-w-0 items-center justify-between gap-2 rounded-md border p-3">
                {href ? (
                  <Link to={href} className="min-w-0 truncate font-medium hover:underline">
                    {title}
                  </Link>
                ) : (
                  <span className="min-w-0 truncate font-medium">{title}</span>
                )}
                {!readOnly && (
                  <Button
                    variant="ghost"
                    size="sm"
                    disabled={busy}
                    aria-label={t('relations.unlinkLabel', { title })}
                    onClick={() => unlinkRecord(item.id)}
                  >
                    <Unlink aria-hidden="true" />
                    <span className="hidden sm:inline">{t('relations.unlink')}</span>
                  </Button>
                )}
              </li>
            )
          })}
        </ul>
      )}
      {pages > 1 && (
        <div className="flex items-center justify-between gap-2 text-sm text-muted-foreground">
          <span>{t('relations.page', { page, pages, total })}</span>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" disabled={page <= 1} onClick={() => setPage((value) => value - 1)}>
              {t('relations.previous')}
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={page >= pages}
              onClick={() => setPage((value) => value + 1)}
            >
              {t('relations.next')}
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}

export interface RelationsTabProps {
  recordType: string
  recordId: string
  readOnly?: boolean
}

export function RelationsTab({ recordType, recordId, readOnly = false }: RelationsTabProps) {
  const { t } = useTranslation('objects')
  const relations = useRecordRelations(recordType, recordId)

  if (relations.isPending) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-11 w-full" />
        <Skeleton className="h-11 w-full" />
      </div>
    )
  }
  if (relations.isError) {
    return <ErrorState title={t('relations.loadError')} onRetry={() => void relations.refetch()} />
  }
  if (relations.data.length === 0) {
    return <EmptyState icon={Link2} title={t('relations.noneTitle')} description={t('relations.none')} />
  }

  return (
    <div className="grid gap-6">
      {relations.data.map((relation) => (
        <section key={relation.key} className="grid gap-3" aria-labelledby={`relation-${relation.key}`}>
          <div className="flex items-center gap-2">
            <h3 id={`relation-${relation.key}`} className="text-sm font-semibold">
              {relation.label}
            </h3>
            {relation.total != null && <Badge variant="secondary">{relation.total}</Badge>}
            {relation.required && <Badge variant="outline">{t('relations.required')}</Badge>}
          </div>
          <RelationPanel recordType={recordType} recordId={recordId} relation={relation} readOnly={readOnly} />
        </section>
      ))}
    </div>
  )
}
