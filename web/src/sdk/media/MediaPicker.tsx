import { useQueryClient } from '@tanstack/react-query'
import { FileText, Upload } from 'lucide-react'
import { useRef, useState } from 'react'
import { errorMessage } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { SearchInput } from '@/components/ui/search-input'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { fileDownloadUrl } from '@/features/files'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { mediaApi, mediaKeys, type MediaAsset, useMediaAssets } from './api'

export function MediaThumbnail({ asset, className }: { asset: MediaAsset; className?: string }) {
  if (asset.file && asset.content_type?.startsWith('image/')) {
    return <img src={fileDownloadUrl(asset.file)} alt="" loading="lazy" className={className} style={{ objectPosition: `${asset.focal_x ?? 50}% ${asset.focal_y ?? 50}%` }} />
  }
  return (
    <span className={className} aria-hidden>
      <FileText />
    </span>
  )
}

interface MediaPickerProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onPick: (asset: MediaAsset) => void | Promise<void>
  images?: boolean
  canUpload?: boolean
  accept?: string
}

export function MediaPicker({ open, onOpenChange, onPick, images = true, canUpload = false, accept }: MediaPickerProps) {
  const t = useModuleTranslation('media')
  const client = useQueryClient()
  const input = useRef<HTMLInputElement>(null)
  const [search, setSearch] = useState('')
  const [busy, setBusy] = useState(false)
  const assets = useMediaAssets(search, images, open)

  async function pick(asset: MediaAsset) {
    setBusy(true)
    try {
      await onPick(asset)
      onOpenChange(false)
    } catch (failure) {
      toast({ title: errorMessage(failure), variant: 'destructive' })
    } finally {
      setBusy(false)
    }
  }

  async function upload(file: File) {
    setBusy(true)
    try {
      const created = await mediaApi.create(file)
      await client.invalidateQueries({ queryKey: mediaKeys.all })
      await pick(created)
    } catch (failure) {
      toast({ title: errorMessage(failure), variant: 'destructive' })
    } finally {
      setBusy(false)
      if (input.current) input.current.value = ''
    }
  }

  const list = assets.data ?? []

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="3xl">
        <DialogHeader>
          <DialogTitle>{t('picker.title')}</DialogTitle>
          <DialogDescription>{t('picker.description')}</DialogDescription>
        </DialogHeader>
        <div className="flex flex-wrap items-center gap-2">
          <SearchInput value={search} onValueChange={setSearch} placeholder={t('picker.search')} aria-label={t('picker.search')} wrapperClassName="flex-1 min-w-48" />
          {canUpload && (
            <>
              <input
                ref={input}
                type="file"
                accept={accept}
                className="hidden"
                tabIndex={-1}
                aria-hidden
                onChange={(event) => {
                  const file = event.target.files?.[0]
                  if (file) void upload(file)
                }}
              />
              <Button type="button" variant="outline" disabled={busy} onClick={() => input.current?.click()}>
                <Upload aria-hidden />
                {t('picker.upload')}
              </Button>
            </>
          )}
        </div>
        {assets.isPending ? (
          <Skeleton className="h-48" />
        ) : list.length === 0 ? (
          <EmptyState title={search ? t('picker.noResults') : t('picker.empty')} />
        ) : (
          <ul className="grid max-h-[60vh] grid-cols-2 gap-3 overflow-y-auto sm:grid-cols-4">
            {list.map((asset) => (
              <li key={asset.id}>
                <button
                  type="button"
                  disabled={busy || !asset.file}
                  className="grid w-full gap-1 rounded-md border bg-card p-1 text-left text-xs hover:border-primary focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none disabled:opacity-60"
                  aria-label={t('picker.pick', { name: asset.name })}
                  onClick={() => void pick(asset)}
                >
                  <MediaThumbnail asset={asset} className="flex aspect-[4/3] w-full items-center justify-center rounded-sm bg-muted object-cover text-muted-foreground" />
                  <span className="truncate px-1">{asset.name}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </DialogContent>
    </Dialog>
  )
}
