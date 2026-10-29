import { NotebookPen } from 'lucide-react'
import { useId, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { cn } from '@/lib/utils'
import {
  useCreateNote,
  useDeleteNote,
  usePinNote,
  useRecordNotes,
  useUpdateNote,
  type Note,
  type NoteTarget,
  type NoteVisibility,
} from './api'
import { NoteCard } from './components/NoteCard'
import { NoteEditor } from './components/NoteEditor'
import { noteErrorKey } from './lib/errors'

export interface NotesTabProps {
  recordType: string
  recordId: string
  readOnly?: boolean
  className?: string
}

export function NotesTab({ recordType, recordId, readOnly = false, className }: NotesTabProps) {
  const { t } = useTranslation('notes')
  const { hasPermission } = useSession()
  const titleId = useId()
  const target = useMemo<NoteTarget>(() => ({ recordType, recordId }), [recordType, recordId])
  const notes = useRecordNotes(target)
  const create = useCreateNote(target)
  const update = useUpdateNote(target)
  const pin = usePinNote(target)
  const remove = useDeleteNote(target)
  const canCreate = !readOnly && hasPermission('notes.create')

  const fail = (error: unknown) => toast({ title: t(noteErrorKey(error)), variant: 'destructive' })

  const onCreate = async (body: string, visibility: NoteVisibility) => {
    try {
      await create.mutateAsync({ body, visibility })
      toast({ title: t('tab.created'), variant: 'success' })
    } catch (error) {
      fail(error)
      throw error
    }
  }

  const onSave = async (note: Note, body: string, visibility: NoteVisibility) => {
    try {
      await update.mutateAsync({
        id: note.id,
        request: visibility === note.visibility ? { body } : { body, visibility },
      })
      toast({ title: t('tab.updated'), variant: 'success' })
    } catch (error) {
      fail(error)
      throw error
    }
  }

  const onTogglePin = (note: Note) =>
    pin.mutate({ id: note.id, pinned: !note.pinned }, { onError: fail })

  const onDelete = async (note: Note) => {
    try {
      await remove.mutateAsync(note.id)
      toast({ title: t('tab.deleted'), variant: 'success' })
    } catch (error) {
      fail(error)
    }
  }

  const items = notes.data?.data ?? []
  const busy = update.isPending || pin.isPending

  return (
    <section aria-labelledby={titleId} className={cn('flex flex-col gap-4', className)}>
      <header className="flex items-center gap-2">
        <NotebookPen className="size-4 text-muted-foreground" aria-hidden="true" />
        <h2 id={titleId} className="text-base font-semibold">
          {t('tab.title')}
        </h2>
        {notes.isSuccess && (
          <span className="text-sm text-muted-foreground">{t('tab.count', { count: notes.data.meta.total })}</span>
        )}
      </header>
      {canCreate && (
        <div className="rounded-md border p-3">
          <NoteEditor
            submitLabel={t('tab.add')}
            pending={create.isPending}
            subject={{ recordType, recordId }}
            onSubmit={(value) => onCreate(value.body, value.visibility)}
          />
        </div>
      )}
      {notes.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-20 w-full" />
          <Skeleton className="h-20 w-full" />
        </div>
      ) : notes.isError ? (
        <ErrorState onRetry={() => void notes.refetch()} />
      ) : items.length === 0 ? (
        <EmptyState icon={NotebookPen} title={t('tab.emptyTitle')} description={t('tab.emptyDescription')} />
      ) : (
        <ul className="grid gap-3" aria-label={t('tab.listLabel')}>
          {items.map((note) => (
            <NoteCard
              key={note.id}
              note={note}
              pending={busy}
              onSave={onSave}
              onTogglePin={onTogglePin}
              onDelete={onDelete}
            />
          ))}
        </ul>
      )}
    </section>
  )
}
