import { ArrowRight, Pencil, Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { notifyObjectsError, useRelationships, type Relationship } from '@/features/custom-records'
import { objectsAdminApi, useInvalidateObjects, type CustomObjectAdmin } from '@/modules/objects/ui/api'
import { useRecordTypeLabels } from '@/modules/objects/ui/labels'
import { RelationshipEditorDialog } from '@/modules/objects/ui/RelationshipEditorDialog'
import { EmptyState } from '@/components/ui/empty-state'

export interface RelationshipsSectionProps {
  object: CustomObjectAdmin
}

export function RelationshipsSection({ object }: RelationshipsSectionProps) {
  const { t } = useTranslation('objects')
  const relationships = useRelationships(object.record_type)
  const invalidate = useInvalidateObjects()
  const { label } = useRecordTypeLabels()
  const [editor, setEditor] = useState<{ relationship: Relationship | null } | null>(null)
  const [deleting, setDeleting] = useState<Relationship | null>(null)

  const remove = async (relationship: Relationship) => {
    try {
      await objectsAdminApi.removeRelationship(relationship.key)
    } catch (error) {
      notifyObjectsError(error)
      throw error
    }
    notifySuccess(t('relationships.deleted'))
    void invalidate()
  }

  return (
    <section className="grid gap-3" aria-labelledby="object-relationships">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="grid gap-1">
          <h3 id="object-relationships" className="text-base font-semibold">
            {t('relationships.title')}
          </h3>
          <p className="text-sm text-muted-foreground">{t('relationships.description')}</p>
        </div>
        <Button size="sm" onClick={() => setEditor({ relationship: null })}>
          <Plus aria-hidden="true" />
          {t('relationships.new')}
        </Button>
      </div>
      {relationships.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-16 w-full" />
        </div>
      ) : relationships.isError ? (
        <ErrorState title={t('relationships.loadError')} onRetry={() => void relationships.refetch()} />
      ) : relationships.data.length === 0 ? (
        <EmptyState title={t('relationships.empty')} />
      ) : (
        <ul className="grid gap-2" aria-label={t('relationships.title')}>
          {relationships.data.map((relationship) => (
            <li key={relationship.key} className="grid gap-2 rounded-md border bg-card p-3">
              <div className="flex min-w-0 flex-wrap items-center justify-between gap-2">
                <div className="flex min-w-0 flex-wrap items-center gap-1.5 text-sm font-medium">
                  <span className="truncate">{label(relationship.from_type)}</span>
                  <ArrowRight className="size-4 shrink-0 text-muted-foreground" aria-label={t('relationships.arrow')} />
                  <span className="truncate">{label(relationship.to_type)}</span>
                </div>
                <div className="flex items-center gap-1">
                  <Button
                    size="sm"
                    variant="ghost"
                    aria-label={t('relationships.editLabel', { key: relationship.key })}
                    onClick={() => setEditor({ relationship })}
                  >
                    <Pencil aria-hidden="true" />
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    aria-label={t('relationships.deleteLabel', { key: relationship.key })}
                    onClick={() => setDeleting(relationship)}
                  >
                    <Trash2 aria-hidden="true" />
                  </Button>
                </div>
              </div>
              <p className="text-xs text-muted-foreground">
                {t('relationships.summary', {
                  from: label(relationship.from_type),
                  fromLabel: relationship.from_label,
                  to: label(relationship.to_type),
                  toLabel: relationship.to_label,
                })}
              </p>
              <div className="flex flex-wrap items-center gap-1.5">
                <Badge variant="outline">{t(`relationships.cardinalities.${relationship.cardinality}.title`)}</Badge>
                {relationship.required && <Badge variant="secondary">{t('relationships.requiredBadge')}</Badge>}
                <Badge variant="outline">{t(`relationships.onDeleteBadges.${relationship.on_delete}`)}</Badge>
                {relationship.link_count != null && (
                  <span className="text-xs text-muted-foreground">
                    {t('relationships.linkCount', { count: relationship.link_count })}
                  </span>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
      <RelationshipEditorDialog
        relationship={editor?.relationship ?? null}
        recordType={object.record_type}
        open={editor !== null}
        onOpenChange={(open) => !open && setEditor(null)}
        onSaved={() => void invalidate()}
      />
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        variant="destructive"
        title={t('relationships.deleteTitle')}
        description={t('relationships.deleteDescription', { count: deleting?.link_count ?? 0 })}
        confirmLabel={t('relationships.delete')}
        notifyOnError={false}
        onConfirm={() => (deleting ? remove(deleting) : undefined)}
      />
    </section>
  )
}
