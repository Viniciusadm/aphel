import { CircleAlert, Download, FileText, Paperclip, Trash2, Upload, X } from 'lucide-react'
import { useId, useMemo, useRef, useState, type ChangeEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { cn } from '@/lib/utils'
import {
  fileDownloadUrl,
  useAttachments,
  useRemoveAttachment,
  useUploadAttachment,
  type Attachment,
  type RecordTarget,
} from './api'
import { filesErrorKey, type FilesMessageKey } from './lib/errors'
import { formatDateTime, formatFileSize } from './lib/format'

export interface AttachmentsPanelProps {
  recordType: string
  recordId: string
  readOnly?: boolean
  className?: string
}

interface UploadState {
  id: number
  name: string
  progress: number
  error?: FilesMessageKey
}

export function AttachmentsPanel({ recordType, recordId, readOnly = false, className }: AttachmentsPanelProps) {
  const { t } = useTranslation('files')
  const titleId = useId()
  const target = useMemo<RecordTarget>(() => ({ recordType, recordId }), [recordType, recordId])
  const attachments = useAttachments(target)
  const upload = useUploadAttachment(target)
  const remove = useRemoveAttachment(target)
  const inputRef = useRef<HTMLInputElement>(null)
  const nextUploadId = useRef(1)
  const [uploads, setUploads] = useState<UploadState[]>([])
  const [removing, setRemoving] = useState<Attachment | null>(null)

  const updateUpload = (id: number, changes: Partial<UploadState>) =>
    setUploads((current) => current.map((item) => (item.id === id ? { ...item, ...changes } : item)))
  const dropUpload = (id: number) => setUploads((current) => current.filter((item) => item.id !== id))

  const startUpload = async (file: File) => {
    const id = nextUploadId.current++
    setUploads((current) => [...current, { id, name: file.name, progress: 0 }])
    try {
      await upload.mutateAsync({ file, onProgress: (fraction) => updateUpload(id, { progress: fraction }) })
      dropUpload(id)
      toast({ title: t('upload.success', { name: file.name }), variant: 'success' })
    } catch (error) {
      updateUpload(id, { error: filesErrorKey(error) })
    }
  }

  const onFilesSelected = (event: ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(event.target.files ?? [])
    event.target.value = ''
    for (const file of files) void startUpload(file)
  }

  const confirmRemoval = async () => {
    if (!removing) return
    try {
      await remove.mutateAsync(removing.id)
      toast({ title: t('remove.success'), variant: 'success' })
    } catch (error) {
      toast({ title: t(filesErrorKey(error)), variant: 'destructive' })
    } finally {
      setRemoving(null)
    }
  }

  const total = attachments.data?.meta.total ?? 0

  return (
    <section aria-labelledby={titleId} className={cn('flex flex-col gap-4', className)}>
      <header className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <Paperclip className="size-4 text-muted-foreground" aria-hidden />
          <h2 id={titleId} className="text-base font-semibold">
            {t('panel.title')}
          </h2>
          {attachments.isSuccess && (
            <span className="text-sm text-muted-foreground">{t('panel.count', { count: total })}</span>
          )}
        </div>
        {!readOnly && (
          <>
            <input
              ref={inputRef}
              type="file"
              multiple
              className="sr-only"
              aria-label={t('panel.fileInput')}
              onChange={onFilesSelected}
              tabIndex={-1}
            />
            <Button size="sm" variant="outline" onClick={() => inputRef.current?.click()}>
              <Upload aria-hidden />
              {t('panel.upload')}
            </Button>
          </>
        )}
      </header>

      {uploads.length > 0 && (
        <ul className="flex flex-col gap-2">
          {uploads.map((item) => (
            <UploadRow key={item.id} upload={item} onDismiss={() => dropUpload(item.id)} />
          ))}
        </ul>
      )}

      <AttachmentList
        attachments={attachments.data?.data ?? []}
        loading={attachments.isPending}
        failed={attachments.isError}
        onRetry={() => void attachments.refetch()}
        onRemove={readOnly ? undefined : setRemoving}
      />

      <Dialog open={removing !== null} onOpenChange={(open) => !open && setRemoving(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('remove.title')}</DialogTitle>
            <DialogDescription>{t('remove.description', { name: removing?.file.original_name ?? '' })}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRemoving(null)} disabled={remove.isPending}>
              {t('remove.cancel')}
            </Button>
            <Button variant="destructive" onClick={() => void confirmRemoval()} disabled={remove.isPending}>
              {t('remove.confirm')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  )
}

interface UploadRowProps {
  upload: UploadState
  onDismiss: () => void
}

function UploadRow({ upload, onDismiss }: UploadRowProps) {
  const { t } = useTranslation('files')
  const percent = Math.round(upload.progress * 100)
  if (upload.error) {
    return (
      <li role="alert" className="flex items-start gap-2 rounded-md border border-destructive/40 p-3 text-sm">
        <CircleAlert className="mt-0.5 size-4 text-destructive" aria-hidden />
        <div className="grid flex-1 gap-0.5">
          <span className="font-medium break-all">{t('upload.failed', { name: upload.name })}</span>
          <span className="text-muted-foreground">{t(upload.error)}</span>
        </div>
        <Button size="icon" variant="ghost" className="size-7" onClick={onDismiss} aria-label={t('upload.dismiss', { name: upload.name })}>
          <X aria-hidden />
        </Button>
      </li>
    )
  }
  return (
    <li className="grid gap-1.5 rounded-md border p-3 text-sm">
      <div className="flex items-center justify-between gap-2">
        <span className="truncate">{t('upload.uploading', { name: upload.name })}</span>
        <span className="text-muted-foreground tabular-nums">{percent}%</span>
      </div>
      <div
        role="progressbar"
        aria-label={t('upload.progress', { name: upload.name })}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
        className="h-1.5 overflow-hidden rounded-full bg-muted"
      >
        <div className="h-full bg-primary transition-[width]" style={{ width: `${percent}%` }} />
      </div>
    </li>
  )
}

interface AttachmentListProps {
  attachments: Attachment[]
  loading: boolean
  failed: boolean
  onRetry: () => void
  onRemove?: (attachment: Attachment) => void
}

function AttachmentList({ attachments, loading, failed, onRetry, onRemove }: AttachmentListProps) {
  const { t, i18n } = useTranslation('files')
  if (loading) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-14" />
        <Skeleton className="h-14" />
      </div>
    )
  }
  if (failed) return <ErrorState onRetry={onRetry} />
  if (attachments.length === 0) {
    return <EmptyState icon={Paperclip} title={t('panel.emptyTitle')} description={t('panel.emptyDescription')} />
  }
  return (
    <ul aria-label={t('panel.listLabel')} className="divide-y rounded-md border">
      {attachments.map((attachment) => {
        const name = attachment.file.original_name
        const author = attachment.created_by?.display_name ?? attachment.file.uploaded_by?.display_name
        return (
          <li key={attachment.id} className="flex items-center gap-3 p-3">
            <FileText className="size-5 shrink-0 text-muted-foreground" aria-hidden />
            <div className="grid min-w-0 flex-1 gap-0.5">
              <a
                href={fileDownloadUrl(attachment.file.id)}
                className="truncate text-sm font-medium hover:underline"
                download={name}
              >
                {name}
              </a>
              <p className="text-xs text-muted-foreground">
                {formatFileSize(attachment.file.size, i18n.language)}
                {' · '}
                {t('panel.uploadedBy', { name: author ?? t('panel.unknownAuthor') })}
                {' · '}
                <time dateTime={attachment.created_at}>{formatDateTime(attachment.created_at, i18n.language)}</time>
              </p>
            </div>
            <Button asChild size="icon" variant="ghost" className="size-8">
              <a href={fileDownloadUrl(attachment.file.id)} download={name} aria-label={t('panel.download', { name })}>
                <Download aria-hidden />
              </a>
            </Button>
            {onRemove && (
              <Button
                size="icon"
                variant="ghost"
                className="size-8 text-destructive hover:text-destructive"
                onClick={() => onRemove(attachment)}
                aria-label={t('panel.remove', { name })}
              >
                <Trash2 aria-hidden />
              </Button>
            )}
          </li>
        )
      })}
    </ul>
  )
}
