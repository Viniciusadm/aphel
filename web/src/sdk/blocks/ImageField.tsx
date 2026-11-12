import { api } from '@/api'
import { errorMessage } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/ui/confirm-dialog'
import { Field } from '@/components/ui/field'
import { toast } from '@/components/ui/toast-store'
import { fileDownloadUrl, uploadFile, useAttachments } from '@/features/files'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { useQueryClient } from '@tanstack/react-query'
import { Images, ImagePlus, Trash2, X } from 'lucide-react'
import { useSession } from '@/app/session/session-context'
import { MEDIA_MANAGE, MEDIA_READ, mediaApi, mediaKeys, type MediaAsset } from '../media/api'
import { MediaPicker } from '../media/MediaPicker'
import { useId, useRef, useState } from 'react'

import { useImagesInUse } from './imageUsage'
import type { BlockRecord } from './api'
import { IMAGE_TYPES } from './blocks'

interface ImageFieldProps {
  label?: string
  record: BlockRecord
  value: string | null
  onChange: (value: string | null) => void
  onAsset?: (asset: MediaAsset) => void
  disabled?: boolean
}

export function ImageField({ label, record, value, onChange, onAsset, disabled }: ImageFieldProps) {
  const { hasPermission } = useSession()
  const library = hasPermission(MEDIA_READ)
  const addToLibrary = hasPermission(MEDIA_MANAGE)
  const [picking, setPicking] = useState(false)
  const t = useModuleTranslation('site-blocks')
  const client = useQueryClient()
  const input = useRef<HTMLInputElement>(null)
  const libraryId = useId()
  const [uploading, setUploading] = useState(false)
  const [removing, setRemoving] = useState<{ id: string; name: string } | null>(null)
  const inUse = useImagesInUse()
  const attachments = useAttachments(record)
  const images = (attachments.data?.data ?? []).filter((attachment) => IMAGE_TYPES.includes(attachment.file.mime_type))
  const current = images.find((attachment) => attachment.file.id === value)

  async function upload(file: File) {
    if (!IMAGE_TYPES.includes(file.type)) {
      toast({ title: t('images.onlyImages'), variant: 'destructive' })
      return
    }
    setUploading(true)
    try {
      if (addToLibrary) {
        const asset = await mediaApi.create(file)
        await choose(asset)
        await client.invalidateQueries({ queryKey: mediaKeys.all })
      } else {
        const uploaded = await uploadFile({ file, target: record })
        await client.invalidateQueries({ queryKey: ['files'] })
        onChange(uploaded.file.id)
      }
    } catch (failure) {
      toast({ title: errorMessage(failure), variant: 'destructive' })
    } finally {
      setUploading(false)
      if (input.current) input.current.value = ''
    }
  }

  async function choose(asset: MediaAsset) {
    const file = await mediaApi.attachTo(asset, record)
    await client.invalidateQueries({ queryKey: ['files'] })
    onChange(file)
    onAsset?.(asset)
  }

  const others = images.filter((attachment) => attachment.file.id !== value)

  return (
    <Field label={label ?? t('fields.image')}>
      {() => (
        <div className="blocks-image-field">
          {value && (
            <div className="blocks-image-current">
              {current ? <img src={fileDownloadUrl(value)} alt="" /> : <p className="blocks-muted">{t('images.missing')}</p>}
            </div>
          )}
          <div className="blocks-row">
            <input
              ref={input}
              type="file"
              accept={IMAGE_TYPES.join(',')}
              className="blocks-hidden"
              tabIndex={-1}
              aria-hidden
              onChange={(event) => {
                const file = event.target.files?.[0]
                if (file) void upload(file)
              }}
            />
            <Button type="button" variant="outline" size="sm" disabled={disabled || uploading} onClick={() => input.current?.click()}>
              <ImagePlus aria-hidden />
              {uploading ? t('images.uploading') : value ? t('images.replace') : t('images.choose')}
            </Button>
            {library && (
              <Button type="button" variant="outline" size="sm" disabled={disabled || uploading} onClick={() => setPicking(true)}>
                <Images aria-hidden />
                {t('images.fromLibrary')}
              </Button>
            )}
            {value && (
              <Button type="button" variant="ghost" size="sm" disabled={disabled} onClick={() => onChange(null)}>
                <X aria-hidden />
                {t('images.remove')}
              </Button>
            )}
          </div>
          {others.length > 0 && !disabled && (
            <div>
              <p id={libraryId} className="blocks-muted">
                {t('images.library')}
              </p>
              <ul className="blocks-library" aria-labelledby={libraryId}>
                {others.map((attachment) => (
                  <li key={attachment.id} className="blocks-library-entry">
                    {!inUse.has(attachment.file.id) && (
                      <button
                        type="button"
                        className="blocks-library-remove"
                        aria-label={t('images.delete', { name: attachment.file.original_name })}
                        title={t('images.delete', { name: attachment.file.original_name })}
                        onClick={() => setRemoving({ id: attachment.id, name: attachment.file.original_name })}
                      >
                        <Trash2 aria-hidden size={12} />
                      </button>
                    )}
                    <button
                      type="button"
                      className="blocks-library-item"
                      aria-label={t('images.pick', { name: attachment.file.original_name })}
                      title={attachment.file.original_name}
                      onClick={() => onChange(attachment.file.id)}
                    >
                      <img src={fileDownloadUrl(attachment.file.id)} alt="" loading="lazy" />
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          )}
          {library && <MediaPicker open={picking} onOpenChange={setPicking} onPick={choose} canUpload={addToLibrary} accept={IMAGE_TYPES.join(',')} />}
          <ConfirmDialog
            open={removing !== null}
            onOpenChange={(open) => !open && setRemoving(null)}
            title={t('images.deleteTitle')}
            description={t('images.deleteDescription', { name: removing?.name ?? '' })}
            confirmLabel={t('images.deleteConfirm')}
            variant="destructive"
            onConfirm={async () => {
              if (!removing) return
              await api.delete(`/v1/attachments/${encodeURIComponent(removing.id)}`)
              await client.invalidateQueries({ queryKey: ['files'] })
              toast({ title: t('images.deleted'), variant: 'success' })
            }}
          />
        </div>
      )}
    </Field>
  )
}
