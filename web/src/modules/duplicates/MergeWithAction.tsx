import { GitMerge } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { RecordFieldInput } from '@/components/fields/RecordFieldInput'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import type { RecordSlotProps } from '@/kernel/modules/slots'
import { useDuplicateTypes } from './api'
import { mergePagePath } from './paths'

export function MergeWithAction({ recordType, record, readOnly }: RecordSlotProps) {
  const { t } = useTranslation('duplicates')
  const { hasPermission } = useSession()
  const navigate = useNavigate()
  const types = useDuplicateTypes()
  const inputId = useId()
  const [open, setOpen] = useState(false)
  const [duplicateId, setDuplicateId] = useState<string | null>(null)
  const info = types.data?.find((type) => type.record_type === recordType)
  const allowed = info && hasPermission(info.permissions.update) && hasPermission(info.permissions.delete)
  if (readOnly || !allowed) return null

  const close = (next: boolean) => {
    setOpen(next)
    if (!next) setDuplicateId(null)
  }
  const valid = duplicateId !== null && duplicateId !== record.id

  return (
    <>
      <Button size="sm" variant="outline" onClick={() => setOpen(true)}>
        <GitMerge aria-hidden="true" />
        {t('mergeWith.action')}
      </Button>
      <Dialog open={open} onOpenChange={close}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('mergeWith.title')}</DialogTitle>
            <DialogDescription>{t('mergeWith.description')}</DialogDescription>
          </DialogHeader>
          <form
            className="grid gap-4"
            noValidate
            onSubmit={(event) => {
              event.preventDefault()
              if (!valid) return
              close(false)
              void navigate(mergePagePath(recordType, record.id, duplicateId))
            }}
          >
            <div className="grid gap-2.5">
              <Label htmlFor={inputId}>{t('mergeWith.label')}</Label>
              <RecordFieldInput
                id={inputId}
                targetRecordType={recordType}
                value={duplicateId}
                onChange={setDuplicateId}
                invalid={duplicateId !== null && !valid}
              />
            </div>
            <DialogFooter>
              <Button type="submit" disabled={!valid}>
                {t('mergeWith.submit')}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  )
}
