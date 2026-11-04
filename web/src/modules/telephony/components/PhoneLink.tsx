import { useContext, useState, type MouseEvent, type SyntheticEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { SessionContext } from '@/app/session/session-context'
import { toast } from '@/components/ui/toast-store'
import { useDisplayRecord } from '@/features/records/display/display-record'
import type { FieldDisplayProps } from '@/kernel/modules/field-types'
import { callsApi, useTelephonyProviders } from '../api'
import { callErrorKey } from '../lib/errors'
import type { StartedCall } from '../types'
import { LogCallDialog } from './LogCallDialog'

function stop(event: SyntheticEvent) {
  event.stopPropagation()
}

export function PhoneLink({ value }: FieldDisplayProps) {
  const phone = String(value)
  const record = useDisplayRecord()
  const { t } = useTranslation('telephony')
  const session = useContext(SessionContext)
  const canLog = session?.hasPermission('tasks.create') ?? false
  const providers = useTelephonyProviders(canLog)
  const [open, setOpen] = useState(false)
  const [started, setStarted] = useState<StartedCall | null>(null)

  const onClick = (event: MouseEvent<HTMLAnchorElement>) => {
    event.stopPropagation()
    if (!canLog) return
    const provider = providers.data?.[0]
    if (!provider) {
      setStarted(null)
      setOpen(true)
      return
    }
    event.preventDefault()
    void callsApi
      .start({
        provider: provider.key,
        phone_number: phone,
        record_type: record?.recordType ?? null,
        record_id: record?.recordId ?? null,
      })
      .then((call) => {
        setStarted(call)
        setOpen(true)
      })
      .catch((error: unknown) => toast({ title: t(callErrorKey(error)), variant: 'destructive' }))
  }

  return (
    <>
      <a
        href={`tel:${phone}`}
        onClick={onClick}
        title={canLog ? t('phone.callAndLog') : undefined}
        className="truncate underline-offset-4 hover:underline"
      >
        {phone}
      </a>
      {canLog && (
        <span className="contents" onClick={stop} onKeyDown={stop} onPointerDown={stop}>
          <LogCallDialog open={open} onOpenChange={setOpen} record={record} phoneNumber={phone} started={started} />
        </span>
      )}
    </>
  )
}
