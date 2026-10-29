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
import { CopyableLink } from '../email/CopyableLink'

export interface MemberLink {
  title: string
  description: string
  link: string
}

interface MemberLinkDialogProps {
  value: MemberLink
  onClose: () => void
}

export function MemberLinkDialog({ value, onClose }: MemberLinkDialogProps) {
  const { t } = useTranslation('settings')
  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{value.title}</DialogTitle>
          <DialogDescription>{value.description}</DialogDescription>
        </DialogHeader>
        <CopyableLink link={value.link} label={t('users.linkDialog.label')} description={t('users.linkDialog.hint')} />
        <DialogFooter>
          <Button onClick={onClose}>{t('users.linkDialog.close')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
