import { useQueryClient } from '@tanstack/react-query'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback/notify'
import type { CreateNoteRequest, Note } from './api'
import { notesKeys } from './api'
import { recordsKeys } from '@/features/records/api'
import { submitWithOutbox } from '@/pwa/offline/outbox'
import { DictationField } from '@/features/mobile/components/DictationField'
import { RecordPicker, type PickedRecord } from '@/features/mobile/components/RecordPicker'
import { useOutboxFeedback } from '@/features/mobile/lib/outbox-feedback'
import { FormActions, FormBody, type QuickFormProps } from '@/features/mobile/components/QuickForm'

export const NOTE_MAX_LENGTH = 50_000

export interface NoteQuickFormProps extends QuickFormProps {
  initialRecord: PickedRecord | null
}

export function NoteQuickForm({ onDone, initialRecord }: NoteQuickFormProps) {
  const { t } = useTranslation('notes', { keyPrefix: 'quickAdd' })
  const queryClient = useQueryClient()
  const feedback = useOutboxFeedback()
  const bodyId = useId()
  const [body, setBody] = useState('')
  const [record, setRecord] = useState<PickedRecord | null>(initialRecord)
  const [touched, setTouched] = useState(false)
  const [pending, setPending] = useState(false)
  const bodyMissing = body.trim() === ''

  const submit = async () => {
    setTouched(true)
    if (bodyMissing || !record) return
    setPending(true)
    try {
      const request: CreateNoteRequest = { record_type: record.recordType, record_id: record.recordId, body: body.trim() }
      const result = await submitWithOutbox<Note>({
        kind: 'note',
        path: '/v1/notes',
        body: request,
        label: t('note.outboxLabel', { title: record.title }),
      })
      feedback(result, t('note.created', { title: record.title }))
      void queryClient.invalidateQueries({ queryKey: notesKeys.forRecord(record) })
      void queryClient.invalidateQueries({ queryKey: recordsKeys.detail(record.recordType, record.recordId) })
      onDone()
    } catch (error) {
      notifyError(error)
    } finally {
      setPending(false)
    }
  }

  return (
    <FormBody onSubmit={() => void submit()}>
      <RecordPicker value={record} onChange={setRecord} required />
      {touched && !record && <p className="-mt-2 text-sm text-destructive">{t('note.recordRequired')}</p>}
      <DictationField
        id={bodyId}
        label={t('note.body')}
        value={body}
        onChange={setBody}
        placeholder={t('note.placeholder')}
        maxLength={NOTE_MAX_LENGTH}
      />
      {touched && bodyMissing && <p className="-mt-2 text-sm text-destructive">{t('note.bodyRequired')}</p>}
      <FormActions pending={pending} label={t('note.submit')} />
    </FormBody>
  )
}
