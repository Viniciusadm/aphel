import { useState } from 'react'
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
import type { InboxItem } from '@/modules/activities/api'
import { useOrganizationUsers } from '@/features/records/api'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { Field } from '@/components/ui/field'

export interface ReassignDialogProps {
  item: InboxItem | null
  onOpenChange: (open: boolean) => void
  onConfirm: (item: InboxItem, ownerId: string) => void
}

export function ReassignDialog({ item, onOpenChange, onConfirm }: ReassignDialogProps) {
  const { t } = useTranslation('activities', { keyPrefix: 'inbox' })
  return (
    <Dialog open={item !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('reassign.title')}</DialogTitle>
          <DialogDescription>{t('reassign.description', { title: item?.title ?? '' })}</DialogDescription>
        </DialogHeader>
        {item && <ReassignForm key={item.id} item={item} onCancel={() => onOpenChange(false)} onConfirm={onConfirm} />}
      </DialogContent>
    </Dialog>
  )
}

function ReassignForm({
  item,
  onCancel,
  onConfirm,
}: {
  item: InboxItem
  onCancel: () => void
  onConfirm: (item: InboxItem, ownerId: string) => void
}) {
  const { t } = useTranslation('activities', { keyPrefix: 'inbox' })
  const users = useOrganizationUsers()
  const [ownerId, setOwnerId] = useState(item.owner_id ?? '')
  const options = users.data ?? []
  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        if (ownerId && ownerId !== item.owner_id) onConfirm(item, ownerId)
        else onCancel()
      }}
    >
      <Field label={t('reassign.owner')}>
        {(control) => <NativeSelect {...control} value={ownerId} disabled={users.isPending} onChange={(event) => setOwnerId(event.target.value)}>
          <option value="" disabled>
            {t('reassign.choose')}
          </option>
          {options.map((user) => (
            <option key={user.id} value={user.id}>
              {user.display_name}
            </option>
          ))}
        </NativeSelect>}
      </Field>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onCancel}>
          {t('reassign.cancel')}
        </Button>
        <Button type="submit" disabled={!ownerId}>
          {t('reassign.confirm')}
        </Button>
      </DialogFooter>
    </form>
  )
}
