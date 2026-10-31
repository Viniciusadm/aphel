import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { useMailboxes } from '@/modules/email/api'
import { MAILBOXES_PATH } from '@/modules/email/constants'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { sequencesApi, useActiveSequences, useSequenceMutation } from '../api'
import { SEQUENCES_PATH } from '../constants'
import { sequenceErrorMessage } from '../lib/errors'
import { Field } from '@/components/ui/field'

export interface EnrollDialogProps {
  contactId: string
  enrolledSequenceIds: string[]
  onClose: () => void
}

export function EnrollDialog({ contactId, enrolledSequenceIds, onClose }: EnrollDialogProps) {
  const { t } = useTranslation('sequences')
  const formId = useId()
  const sequences = useActiveSequences()
  const mailboxes = useMailboxes()
  const [sequenceId, setSequenceId] = useState('')
  const [mailboxId, setMailboxId] = useState('')
  const [failure, setFailure] = useState<string | null>(null)
  const enroll = useSequenceMutation(() =>
    sequencesApi.enroll(sequenceId, { contact_id: contactId, mailbox_id: mailboxId || null }),
  )
  const available = (sequences.data ?? []).filter((sequence) => !enrolledSequenceIds.includes(sequence.id))
  const ownMailboxes = (mailboxes.data ?? []).filter((mailbox) => mailbox.status !== 'disconnected')

  const submit = async () => {
    setFailure(null)
    try {
      await enroll.mutateAsync(undefined)
      notifySuccess(t('record.enrolled'))
      onClose()
    } catch (error) {
      setFailure(sequenceErrorMessage(t, error))
    }
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !enroll.isPending && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('record.dialogTitle')}</DialogTitle>
          <DialogDescription>{t('record.dialogDescription')}</DialogDescription>
        </DialogHeader>
        <form
          id={formId}
          className="grid gap-4"
          noValidate
          onSubmit={(event) => {
            event.preventDefault()
            if (sequenceId) void submit()
          }}
        >
          <Field label={t('record.sequence')}>
            {(control) => <>
              <NativeSelect
              {...control}
              value={sequenceId}
              disabled={sequences.isPending}
              onChange={(event) => setSequenceId(event.target.value)}
            >
              <option value="">{t('record.chooseSequence')}</option>
              {available.map((sequence) => (
                <option key={sequence.id} value={sequence.id}>
                  {sequence.name}
                </option>
              ))}
            </NativeSelect>
            {sequences.isSuccess && available.length === 0 && (
              <p className="text-xs text-muted-foreground">
                {t('record.noSequences')}{' '}
                <Link to={SEQUENCES_PATH} className="underline">
                  {t('record.openSequences')}
                </Link>
              </p>
            )}
            </>}
          </Field>
          <Field label={t('record.mailbox')}>
            {(control) => <>
              <NativeSelect
              {...control}
              value={mailboxId}
              disabled={mailboxes.isPending}
              onChange={(event) => setMailboxId(event.target.value)}
            >
              <option value="">{t('record.defaultMailbox')}</option>
              {ownMailboxes.map((mailbox) => (
                <option key={mailbox.id} value={mailbox.id}>
                  {mailbox.address}
                </option>
              ))}
            </NativeSelect>
            {mailboxes.isSuccess && ownMailboxes.length === 0 && (
              <p className="text-xs text-muted-foreground">
                {t('record.noMailbox')}{' '}
                <Link to={MAILBOXES_PATH} className="underline">
                  {t('record.connectMailbox')}
                </Link>
              </p>
            )}
            </>}
          </Field>
          {failure && (
            <SharedFieldError>
              {failure}
            </SharedFieldError>
          )}
        </form>
        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={enroll.isPending}>
            {t('record.cancel')}
          </Button>
          <Button type="submit" form={formId} disabled={!sequenceId || enroll.isPending}>
            {enroll.isPending ? t('record.enrolling') : t('record.enroll')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
