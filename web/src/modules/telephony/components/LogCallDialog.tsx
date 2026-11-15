import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { DialogForm } from '@/components/ui/dialog-form'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { type DisplayRecord, recordsKeys } from '@/sdk'
import { callsApi, useCallOutcomes, useInvalidateCalls } from '../api'
import { callErrorKey } from '../lib/errors'
import { joinDuration } from '../lib/format'
import type { CallActivity, StartedCall } from '../types'
import { CallForm } from './CallForm'
import { EMPTY_CALL_FORM, type CallFormValues } from './call-form-values'

export interface LogCallDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  record?: DisplayRecord | null
  phoneNumber?: string | null
  started?: StartedCall | null
  onLogged?: (activity: CallActivity) => void
}

export function LogCallDialog({ open, onOpenChange, record, phoneNumber, started, onLogged }: LogCallDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        {open && (
          <LogCallForm
            record={record}
            phoneNumber={phoneNumber}
            started={started}
            onClose={() => onOpenChange(false)}
            onLogged={onLogged}
          />
        )}
      </DialogContent>
    </Dialog>
  )
}

interface LogCallFormProps {
  record?: DisplayRecord | null
  phoneNumber?: string | null
  started?: StartedCall | null
  onClose: () => void
  onLogged?: (activity: CallActivity) => void
}

function LogCallForm({ record, phoneNumber, started, onClose, onLogged }: LogCallFormProps) {
  const { t } = useTranslation('telephony')
  const client = useQueryClient()
  const invalidate = useInvalidateCalls()
  const outcomes = useCallOutcomes()
  const [values, setValues] = useState<CallFormValues>(EMPTY_CALL_FORM)
  const [outcomeError, setOutcomeError] = useState(false)
  const [saving, setSaving] = useState(false)

  const submit = async () => {
    if (!values.outcomeKey) {
      setOutcomeError(true)
      return
    }
    setSaving(true)
    try {
      const activity = await callsApi.log({
        record_type: record?.recordType ?? null,
        record_id: record?.recordId ?? null,
        direction: values.direction,
        outcome_key: values.outcomeKey,
        duration_seconds: joinDuration(values.minutes, values.seconds),
        phone_number: phoneNumber ?? null,
        notes: values.notes.trim() || null,
        provider: started?.provider ?? null,
        provider_call_id: started?.provider_call_id ?? null,
      })
      await invalidate()
      if (record) await client.invalidateQueries({ queryKey: recordsKeys.detail(record.recordType, record.recordId) })
      toast({ title: t('log.logged'), variant: 'success' })
      onLogged?.(activity)
      onClose()
    } catch (error) {
      toast({ title: t(callErrorKey(error)), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  return (
    <DialogForm
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
      title={t('log.title')}
      description={
        record?.title ? t('log.descriptionWith', { name: record.title, phone: phoneNumber ?? '' }) : t('log.description')
      }
      cancelLabel={t('form.cancel')}
      submitLabel={t('log.submit')}
      onCancel={onClose}
      pending={saving}
      submitDisabled={saving || !outcomes.data}
    >
      {outcomes.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-9 w-full" />
          <Skeleton className="h-9 w-full" />
        </div>
      ) : outcomes.isError ? (
        <ErrorState onRetry={() => void outcomes.refetch()} />
      ) : (
        <CallForm
          values={values}
          onChange={(next) => {
            setValues(next)
            if (next.outcomeKey) setOutcomeError(false)
          }}
          outcomes={outcomes.data}
          outcomeError={outcomeError}
          disabled={saving}
          autoStartTimer
        />
      )}
    </DialogForm>
  )
}
