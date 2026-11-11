import { Copy } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import type { Invitation } from '../api'
import { notifyConsoleError, notifyConsoleSuccess } from '../errors'

export function InvitationLinkDialog({ invitation, onClose }: { invitation: Invitation | null; onClose: () => void }) {
  const { t } = useTranslation('console')
  const copy = async () => {
    if (!invitation) return
    try {
      await navigator.clipboard.writeText(invitation.invite_url)
      notifyConsoleSuccess(t('operators.copied'))
    } catch (error) {
      notifyConsoleError(error)
    }
  }

  return (
    <Dialog open={invitation !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('operators.linkTitle')}</DialogTitle>
          <DialogDescription>{t('operators.linkDescription', { name: invitation?.operator.display_name ?? '' })}</DialogDescription>
        </DialogHeader>
        <Field label={t('operators.linkTitle')} hideLabel>
          {(field) => <Input {...field} readOnly value={invitation?.invite_url ?? ''} onFocus={(event) => event.currentTarget.select()} />}
        </Field>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t('operators.close')}
          </Button>
          <Button onClick={() => void copy()}>
            <Copy className="size-4" aria-hidden />
            {t('operators.copy')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
