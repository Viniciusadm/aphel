import { Lock, Pencil, Pin, PinOff, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import type { Note, NoteVisibility } from '../api'
import { Markdown } from '@/components/markdown'
import { useNotePermissions } from '../lib/permissions'
import { NoteEditor } from './NoteEditor'

export interface NoteCardProps {
  note: Note
  pending?: boolean
  onSave: (note: Note, body: string, visibility: NoteVisibility) => Promise<void>
  onTogglePin: (note: Note) => void
  onDelete: (note: Note) => Promise<void>
}

export function NoteCard({ note, pending = false, onSave, onTogglePin, onDelete }: NoteCardProps) {
  const { t } = useTranslation('notes')
  const formatting = useRecordFormatting()
  const { isAuthor, canEdit, canDelete } = useNotePermissions(note)
  const [editing, setEditing] = useState(false)
  const [confirming, setConfirming] = useState(false)
  const edited = note.updated_at !== note.created_at

  return (
    <li
      data-note-id={note.id}
      className="grid gap-2 rounded-md border bg-card px-4 py-3 text-card-foreground"
    >
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
          <span className="font-medium text-foreground">{note.author?.display_name ?? t('card.unknownAuthor')}</span>
          <time dateTime={note.created_at}>{formatDateTime(note.created_at, formatting)}</time>
          {edited && <span>{t('card.edited')}</span>}
          {note.pinned && (
            <Badge variant="secondary">
              <Pin className="size-3" aria-hidden="true" />
              {t('card.pinned')}
            </Badge>
          )}
          {note.visibility === 'private' && (
            <Badge variant="outline" title={t('card.privateHint')}>
              <Lock className="size-3" aria-hidden="true" />
              {t('visibility.private')}
            </Badge>
          )}
        </div>
        {!editing && (canEdit || canDelete) && (
          <div className="flex gap-1">
            {canEdit && (
              <>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={pending}
                  onClick={() => onTogglePin(note)}
                  aria-label={t(note.pinned ? 'card.unpin' : 'card.pin')}
                  title={t(note.pinned ? 'card.unpin' : 'card.pin')}
                >
                  {note.pinned ? <PinOff aria-hidden="true" /> : <Pin aria-hidden="true" />}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setEditing(true)}
                  aria-label={t('card.edit')}
                  title={t('card.edit')}
                >
                  <Pencil aria-hidden="true" />
                </Button>
              </>
            )}
            {canDelete && (
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setConfirming(true)}
                aria-label={t('card.delete')}
                title={t('card.delete')}
              >
                <Trash2 aria-hidden="true" />
              </Button>
            )}
          </div>
        )}
      </div>
      {editing ? (
        <NoteEditor
          initial={{ body: note.body, visibility: note.visibility }}
          allowVisibility={isAuthor}
          submitLabel={t('card.save')}
          pending={pending}
          onCancel={() => setEditing(false)}
          onSubmit={async (value) => {
            await onSave(note, value.body, value.visibility)
            setEditing(false)
          }}
        />
      ) : (
        <Markdown source={note.body} />
      )}
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t('card.deleteTitle')}
        description={t('card.deleteDescription')}
        confirmLabel={t('card.delete')}
        variant="destructive"
        onConfirm={() => onDelete(note)}
      />
    </li>
  )
}
