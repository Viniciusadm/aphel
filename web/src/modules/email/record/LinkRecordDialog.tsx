import { Loader2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback'
import { RecordFieldInput } from '@/components/fields'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { emailApi, useInvalidateThreads } from '../api'
import { useEmailRecordTypes } from '../lib/record-types'
import type { EmailThread } from '../types'
import { Field } from '@/components/ui/field'

interface LinkRecordDialogProps {
  thread: EmailThread
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function LinkRecordDialog({ thread, open, onOpenChange }: LinkRecordDialogProps) {
  const { t } = useTranslation('email')
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('links.title')}</DialogTitle>
          <DialogDescription>{t('links.description')}</DialogDescription>
        </DialogHeader>
        {open && <LinkForm thread={thread} onDone={() => onOpenChange(false)} />}
      </DialogContent>
    </Dialog>
  )
}

function LinkForm({ thread, onDone }: { thread: EmailThread; onDone: () => void }) {
  const { t } = useTranslation('email')
  const recordTypes = useEmailRecordTypes()
  const invalidate = useInvalidateThreads()
  const [recordType, setRecordType] = useState('contacts.contact')
  const [recordId, setRecordId] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const alreadyLinked = thread.records.some(
    (record) => record.record_type === recordType && record.record_id === recordId,
  )

  const save = async () => {
    if (!recordId || alreadyLinked) return
    setSaving(true)
    try {
      await emailApi.linkThread(thread.id, { record_type: recordType, record_id: recordId })
      await invalidate()
      notifySuccess(t('links.linked'))
      onDone()
    } catch (error) {
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      <Field label={t('links.recordType')}>
        {(control) => <NativeSelect
          {...control}
          value={recordType}
          onChange={(event) => {
            setRecordType(event.target.value)
            setRecordId(null)
          }}
        >
          {recordTypes.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </NativeSelect>}
      </Field>
      <Field label={t('links.record')}>
        {(fieldControl) => <>
          <RecordFieldInput
          key={recordType}
          id={fieldControl.id}
          targetRecordType={recordType}
          value={recordId}
          onChange={setRecordId}
        />
        {alreadyLinked && <p className="text-xs text-muted-foreground">{t('links.alreadyLinked')}</p>}
        </>}
      </Field>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onDone}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" disabled={!recordId || alreadyLinked || saving}>
          {saving && <Loader2 className="animate-spin" aria-hidden="true" />}
          {t('links.submit')}
        </Button>
      </DialogFooter>
    </form>
  )
}
