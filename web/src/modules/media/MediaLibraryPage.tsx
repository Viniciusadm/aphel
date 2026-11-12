import { useQueryClient } from '@tanstack/react-query'
import { Upload } from 'lucide-react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/ui/confirm-dialog'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { SearchInput } from '@/components/ui/search-input'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { ToggleField } from '@/components/ui/toggle-field'
import { fileDownloadUrl } from '@/sdk'
import { MEDIA_MANAGE, mediaApi, mediaKeys, type MediaAsset, useMediaAssets, useMediaUsage } from '@/sdk/media/api'
import { MediaThumbnail } from '@/sdk/media/MediaPicker'

function AssetDialog({ asset, canManage, onClose }: { asset: MediaAsset; canManage: boolean; onClose: () => void }) {
  const { t } = useTranslation('media')
  const client = useQueryClient()
  const usage = useMediaUsage(asset.id)
  const [values, setValues] = useState({
    name: asset.name,
    alt: asset.alt ?? '',
    caption: asset.caption ?? '',
    credit: asset.credit ?? '',
    focal_x: asset.focal_x ?? 50,
    focal_y: asset.focal_y ?? 50,
  })
  const [saving, setSaving] = useState(false)
  const [deleting, setDeleting] = useState(false)
  const image = asset.file && asset.content_type?.startsWith('image/')
  const used = usage.data ?? []

  async function save() {
    setSaving(true)
    try {
      await mediaApi.update(asset.id, {
        name: values.name.trim() || asset.name,
        alt: values.alt.trim() || null,
        caption: values.caption.trim() || null,
        credit: values.credit.trim() || null,
        focal_x: values.focal_x,
        focal_y: values.focal_y,
      })
      await client.invalidateQueries({ queryKey: mediaKeys.all })
      notifySuccess(t('edit.saved'))
      onClose()
    } catch (error) {
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent size="2xl">
        <DialogHeader>
          <DialogTitle>{t('edit.title')}</DialogTitle>
        </DialogHeader>
        <div className="grid gap-6 md:grid-cols-2">
          <div className="grid content-start gap-3">
            {image && asset.file ? (
              <img
                src={fileDownloadUrl(asset.file)}
                alt={values.alt}
                className="aspect-video w-full rounded-md border object-cover"
                style={{ objectPosition: `${values.focal_x}% ${values.focal_y}%` }}
              />
            ) : (
              <MediaThumbnail asset={asset} className="flex aspect-video w-full items-center justify-center rounded-md border bg-muted text-muted-foreground" />
            )}
            {image && (
              <Field label={t('edit.focal')} description={t('edit.focalHelp')}>
                {() => (
                  <div className="grid gap-2">
                    <Field label={t('edit.horizontal')}>
                      {(control) => <input {...control} type="range" min={0} max={100} value={values.focal_x} disabled={!canManage} onChange={(event) => setValues({ ...values, focal_x: Number(event.target.value) })} />}
                    </Field>
                    <Field label={t('edit.vertical')}>
                      {(control) => <input {...control} type="range" min={0} max={100} value={values.focal_y} disabled={!canManage} onChange={(event) => setValues({ ...values, focal_y: Number(event.target.value) })} />}
                    </Field>
                  </div>
                )}
              </Field>
            )}
            <div className="grid gap-1">
              <h3 className="text-sm font-medium">{t('edit.usage')}</h3>
              {usage.isPending ? (
                <Skeleton className="h-8" />
              ) : used.length === 0 ? (
                <p className="text-sm text-muted-foreground">{t('edit.notUsed')}</p>
              ) : (
                <ul className="grid gap-1 text-sm">
                  {used.map((item) => (
                    <li key={`${item.record_type}:${item.record_id}`}>
                      <span className="text-muted-foreground">{item.type_label}</span> · {item.label ?? item.record_id}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
          <fieldset className="grid content-start gap-4" disabled={!canManage || saving}>
            <Field label={t('edit.name')}>
              {(control) => <Input {...control} value={values.name} maxLength={160} onChange={(event) => setValues({ ...values, name: event.target.value })} />}
            </Field>
            {image && (
              <Field label={t('edit.alt')} description={t('edit.altHelp')}>
                {(control) => <Textarea {...control} rows={3} value={values.alt} maxLength={300} onChange={(event) => setValues({ ...values, alt: event.target.value })} />}
              </Field>
            )}
            <Field label={t('edit.caption')}>
              {(control) => <Input {...control} value={values.caption} maxLength={300} onChange={(event) => setValues({ ...values, caption: event.target.value })} />}
            </Field>
            <Field label={t('edit.credit')}>
              {(control) => <Input {...control} value={values.credit} maxLength={160} onChange={(event) => setValues({ ...values, credit: event.target.value })} />}
            </Field>
            {canManage && (
              <div className="flex flex-wrap justify-between gap-2 pt-2">
                <Button type="button" variant="ghost" className="text-destructive" disabled={saving || used.length > 0} title={used.length > 0 ? t('edit.inUse') : undefined} onClick={() => setDeleting(true)}>
                  {t('edit.delete')}
                </Button>
                <Button type="button" disabled={saving} onClick={() => void save()}>
                  {t('edit.save')}
                </Button>
              </div>
            )}
            {canManage && used.length > 0 && <p className="text-xs text-muted-foreground">{t('edit.inUse')}</p>}
          </fieldset>
        </div>
        <ConfirmDialog
          open={deleting}
          onOpenChange={setDeleting}
          title={t('edit.deleteTitle', { name: asset.name })}
          description={t('edit.deleteDescription')}
          confirmLabel={t('edit.delete')}
          variant="destructive"
          onConfirm={async () => {
            await mediaApi.remove(asset.id)
            await client.invalidateQueries({ queryKey: mediaKeys.all })
            notifySuccess(t('edit.deleted'))
            onClose()
          }}
        />
      </DialogContent>
    </Dialog>
  )
}

export function MediaLibraryPage() {
  const { t } = useTranslation('media')
  const { hasPermission } = useSession()
  const canManage = hasPermission(MEDIA_MANAGE)
  const client = useQueryClient()
  const input = useRef<HTMLInputElement>(null)
  const [search, setSearch] = useState('')
  const [imagesOnly, setImagesOnly] = useState(false)
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null)
  const [editing, setEditing] = useState<MediaAsset | null>(null)
  const assets = useMediaAssets(search, imagesOnly)
  const list = assets.data ?? []

  async function upload(files: File[]) {
    setProgress({ done: 0, total: files.length })
    let done = 0
    for (const file of files) {
      try {
        await mediaApi.create(file)
        done += 1
        setProgress({ done, total: files.length })
      } catch (error) {
        notifyError(error)
      }
    }
    setProgress(null)
    if (input.current) input.current.value = ''
    await client.invalidateQueries({ queryKey: mediaKeys.all })
    if (done > 0) notifySuccess(t('library.uploaded', { count: done }))
  }

  return (
    <Page>
      <PageHeader
        title={t('library.title')}
        description={t('library.description')}
        actions={
          canManage ? (
            <>
              <input
                ref={input}
                type="file"
                multiple
                className="hidden"
                tabIndex={-1}
                aria-hidden
                onChange={(event) => {
                  const files = [...(event.target.files ?? [])]
                  if (files.length) void upload(files)
                }}
              />
              <Button disabled={progress !== null} onClick={() => input.current?.click()}>
                <Upload aria-hidden />
                {progress ? t('library.uploading', progress) : t('library.upload')}
              </Button>
            </>
          ) : undefined
        }
      />
      <div className="mb-4 flex flex-wrap items-center gap-4">
        <SearchInput value={search} onValueChange={setSearch} placeholder={t('library.search')} aria-label={t('library.search')} wrapperClassName="flex-1 min-w-60" />
        <ToggleField type="switch" label={t('library.onlyImages')} checked={imagesOnly} onCheckedChange={(checked) => setImagesOnly(checked === true)} />
      </div>
      {assets.isPending ? (
        <Skeleton className="h-64" />
      ) : assets.isError ? (
        <ErrorState onRetry={() => void assets.refetch()} />
      ) : list.length === 0 ? (
        <EmptyState title={search ? t('library.noResults') : t('library.empty')} description={search ? undefined : t('library.emptyDescription')} />
      ) : (
        <ul className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-5">
          {list.map((asset) => (
            <li key={asset.id}>
              <button
                type="button"
                className="grid w-full gap-1 rounded-md border bg-card p-1.5 text-left text-sm hover:border-primary focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
                aria-label={t('library.edit', { name: asset.name })}
                onClick={() => setEditing(asset)}
              >
                <MediaThumbnail asset={asset} className="flex aspect-[4/3] w-full items-center justify-center rounded-sm bg-muted object-cover text-muted-foreground" />
                <span className="truncate px-1">{asset.name}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
      {editing && <AssetDialog key={editing.id} asset={editing} canManage={canManage} onClose={() => setEditing(null)} />}
    </Page>
  )
}
