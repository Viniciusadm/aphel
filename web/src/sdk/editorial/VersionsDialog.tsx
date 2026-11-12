import { useEditorialTranslation } from './translation'
import { formatDateTime, useOrganizationUsers, useRecordFormatting } from '../records'
import { Button, Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, EmptyState, PageLoader, StatusBadge, toast } from '../ui'
import { useSession } from '@/app/session/session-context'
import { errorMessage } from '@/components/feedback'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { History } from 'lucide-react'
import { useState } from 'react'

import { revisionChanges, type FieldChange } from './diff'
import { documentKeys, publicationApi, useDocumentKind, type DocumentRecord, type PublicationRevision } from './document'

interface VersionsDialogProps {
  open: boolean
  record: DocumentRecord
  activeRevisionId?: string | null
  onOpenChange: (open: boolean) => void
  beforeRestore: () => Promise<boolean>
  onChanged: (record: DocumentRecord, draftReplaced: boolean) => void
}

export function useRevisions(recordType: string, id: string, enabled: boolean) {
  return useQuery({ queryKey: documentKeys(recordType).revisions(id), queryFn: () => publicationApi(recordType).revisions(id), enabled })
}

function titleOf(revision: PublicationRevision): string {
  const title = revision.title ?? revision.payload?.fields?.draft_title
  return typeof title === 'string' ? title : ''
}

export function VersionsDialog({ open, record: page, activeRevisionId, onOpenChange, beforeRestore, onChanged }: VersionsDialogProps) {
  const kind = useDocumentKind()
  const t = useEditorialTranslation()
  const client = useQueryClient()
  const formatting = useRecordFormatting()
  const users = useOrganizationUsers()
  const { hasPermission } = useSession()
  const canEdit = hasPermission(kind.updatePermission)
  const revisions = useRevisions(kind.recordType, page.id, open)
  const [busy, setBusy] = useState(false)
  const [comparing, setComparing] = useState<string | null>(null)
  const fieldName = (field: string) => {
    const base = field.replace(/^draft_/, '').replace(/_translations$/, '')
    const label = t(`diff.fields.${base}`)
    return label === `pages.diff.fields.${base}` ? base : label
  }
  const describe = (change: FieldChange) => {
    switch (change.kind) {
      case 'text':
        return t('diff.text', { before: change.before || '—', after: change.after || '—' })
      case 'blocks':
        return t('diff.blocks', { added: change.added, removed: change.removed, changed: change.changed })
      case 'languages':
        return t('diff.languages', { languages: change.languages.join(', ') })
      case 'list':
        return t('diff.list', { added: change.added, removed: change.removed })
    }
  }
  const nameOf = (id?: string | null) => (id ? (users.data?.find((user) => user.id === id)?.display_name ?? '—') : '—')

  async function restore(revision: PublicationRevision) {
    setBusy(true)
    try {
      if (!(await beforeRestore())) return
      const latest = await kind.get(page.id)
      const updated = await publicationApi(kind.recordType).restore(revision.id, latest.updated_at)
      await client.invalidateQueries({ queryKey: documentKeys(kind.recordType).revisions(page.id) })
      onChanged(updated, true)
      toast({ title: t('history.restored'), variant: 'success' })
      onOpenChange(false)
    } catch (failure) {
      toast({ title: errorMessage(failure), variant: 'destructive' })
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{t('history.title')}</DialogTitle>
          <DialogDescription>{t('history.description')}</DialogDescription>
        </DialogHeader>
        {revisions.isPending ? (
          <PageLoader />
        ) : !revisions.data?.length ? (
          <EmptyState icon={History} title={t('history.empty')} />
        ) : (
          <ol className="editorial-versions">
            {revisions.data.map((revision) => (
              <li key={revision.id} className="editorial-version">
                <div className="editorial-version-main">
                  <div className="editorial-row">
                    <strong>{t('history.number', { number: revision.number })}</strong>
                    {revision.id === activeRevisionId && <StatusBadge tone="success">{t('history.live')}</StatusBadge>}
                  </div>
                  <span>{titleOf(revision)}</span>
                  <span className="editorial-muted">
                    {t('history.savedBy', { date: formatDateTime(revision.created_at, formatting), name: nameOf(revision.created_by) })}
                  </span>
                  {revision.note && <span className="editorial-version-note">“{revision.note}”</span>}
                </div>
                <div className="editorial-version-actions">
                  <Button type="button" size="sm" variant="ghost" aria-expanded={comparing === revision.id} onClick={() => setComparing(comparing === revision.id ? null : revision.id)}>
                    {t('diff.compare')}
                  </Button>
                  {canEdit && (
                    <Button type="button" size="sm" variant="outline" disabled={busy} onClick={() => void restore(revision)}>
                      {t('history.restore')}
                    </Button>
                  )}
                </div>
                {comparing === revision.id && (
                  <CompareRevision revisionId={revision.id} record={page} describe={describe} fieldName={fieldName} />
                )}
              </li>
            ))}
          </ol>
        )}
      </DialogContent>
    </Dialog>
  )
}

function CompareRevision({ revisionId, record, describe, fieldName }: { revisionId: string; record: DocumentRecord; describe: (change: FieldChange) => string; fieldName: (field: string) => string }) {
  const t = useEditorialTranslation()
  const kind = useDocumentKind()
  const revision = useQuery({ queryKey: [...documentKeys(kind.recordType).all, 'revision', revisionId], queryFn: () => publicationApi(kind.recordType).revision(revisionId) })
  if (revision.isPending) return <p className="editorial-muted">{t('diff.loading')}</p>
  const changes = revisionChanges(revision.data?.payload?.fields ?? {}, record)
  if (changes.length === 0) return <p className="editorial-muted editorial-diff">{t('diff.same')}</p>
  return (
    <div className="editorial-diff">
      <p className="editorial-muted">{t('diff.title')}</p>
      <ul>
        {changes.map((change) => (
          <li key={change.field}>
            <strong>{fieldName(change.field)}</strong>: {describe(change)}
          </li>
        ))}
      </ul>
    </div>
  )
}
