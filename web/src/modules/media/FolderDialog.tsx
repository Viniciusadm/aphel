import { useQueryClient } from '@tanstack/react-query'
import { useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/components/ui/native-select'
import { mediaApi, mediaKeys, type MediaFolder } from '@/sdk/media/api'
import { flattenFolders } from './folders'

interface FolderDialogProps {
  folders: MediaFolder[]
  folder?: MediaFolder
  parentId?: string | null
  onClose: (saved?: MediaFolder) => void
}

export function FolderDialog({ folders, folder, parentId = null, onClose }: FolderDialogProps) {
  const { t } = useTranslation('media')
  const client = useQueryClient()
  const [name, setName] = useState(folder?.name ?? '')
  const [parent, setParent] = useState(folder?.parent_id ?? parentId ?? '')
  const [saving, setSaving] = useState(false)
  const options = flattenFolders(folders, folder?.id)

  async function submit(event: FormEvent) {
    event.preventDefault()
    const clean = name.trim()
    if (!clean) return
    setSaving(true)
    try {
      const saved = folder ? await mediaApi.renameFolder(folder.id, clean) : await mediaApi.createFolder(clean, parent || null)
      await client.invalidateQueries({ queryKey: mediaKeys.all })
      onClose(saved)
    } catch (error) {
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !saving && onClose()}>
      <DialogContent size="md">
        <DialogHeader>
          <DialogTitle>{t(folder ? 'folders.renameTitle' : 'folders.newTitle')}</DialogTitle>
          <DialogDescription>{t('folders.dialogDescription')}</DialogDescription>
        </DialogHeader>
        <form className="grid gap-4" onSubmit={(event) => void submit(event)}>
          <Field label={t('folders.name')}>
            {(control) => <Input {...control} autoFocus required maxLength={80} placeholder={t('placeholders.folder')} value={name} onChange={(event) => setName(event.target.value)} />}
          </Field>
          {!folder && (
            <Field label={t('folders.parent')}>
              {(control) => (
                <NativeSelect {...control} className="w-full" value={parent} onValueChange={setParent}>
                  <option value="">{t('folders.root')}</option>
                  {options.map((item) => (
                    <option key={item.id} value={item.id}>
                      {'  '.repeat(item.depth)}
                      {item.name}
                    </option>
                  ))}
                </NativeSelect>
              )}
            </Field>
          )}
          <DialogFooter>
            <Button type="button" variant="outline" disabled={saving} onClick={() => onClose()}>
              {t('folders.cancel')}
            </Button>
            <Button type="submit" disabled={saving || !name.trim()}>
              {t(folder ? 'folders.save' : 'folders.create')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
