import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/ui/confirm-dialog'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/components/ui/native-select'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { fileDownloadUrl } from '@/sdk'
import { mediaApi, mediaKeys, type MediaAsset, useMediaFolders, useMediaUsage } from '@/sdk/media/api'
import { MediaThumbnail } from '@/sdk/media/MediaPicker'
import { flattenFolders } from './folders'

export function AssetDialog({ asset, canManage, onClose }: { asset: MediaAsset; canManage: boolean; onClose: () => void }) {
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
    folder_id: asset.folder_id ?? '',
  })
  const folders = flattenFolders(useMediaFolders().data ?? [])
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
        folder_id: values.folder_id || null,
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
              {(control) => <Input {...control} placeholder={t('placeholders.name')} value={values.name} maxLength={160} onChange={(event) => setValues({ ...values, name: event.target.value })} />}
            </Field>
            {image && (
              <Field label={t('edit.alt')} description={t('edit.altHelp')}>
                {(control) => <Textarea {...control} rows={3} placeholder={t('placeholders.alt')} value={values.alt} maxLength={300} onChange={(event) => setValues({ ...values, alt: event.target.value })} />}
              </Field>
            )}
            <Field label={t('edit.folder')}>
              {(control) => (
                <NativeSelect {...control} className="w-full" value={values.folder_id} onValueChange={(value) => setValues({ ...values, folder_id: value })}>
                  <option value="">{t('folders.root')}</option>
                  {folders.map((folder) => (
                    <option key={folder.id} value={folder.id}>
                      {'\u00a0\u00a0'.repeat(folder.depth)}{folder.name}
                    </option>
                  ))}
                </NativeSelect>
              )}
            </Field>
            <Field label={t('edit.caption')}>
              {(control) => <Input {...control} placeholder={t('placeholders.caption')} value={values.caption} maxLength={300} onChange={(event) => setValues({ ...values, caption: event.target.value })} />}
            </Field>
            <Field label={t('edit.credit')}>
              {(control) => <Input {...control} placeholder={t('placeholders.credit')} value={values.credit} maxLength={160} onChange={(event) => setValues({ ...values, credit: event.target.value })} />}
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

