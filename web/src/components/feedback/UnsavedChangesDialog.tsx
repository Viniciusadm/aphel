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

export interface UnsavedChangesDialogProps {
  open: boolean
  onStay: () => void
  onLeave: () => void
}

export function UnsavedChangesDialog({ open, onStay, onLeave }: UnsavedChangesDialogProps) {
  const { t } = useTranslation('records-detail')
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onStay()
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('unsaved.title')}</DialogTitle>
          <DialogDescription>{t('unsaved.description')}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={onStay}>
            {t('unsaved.stay')}
          </Button>
          <Button variant="destructive" onClick={onLeave}>
            {t('unsaved.leave')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
