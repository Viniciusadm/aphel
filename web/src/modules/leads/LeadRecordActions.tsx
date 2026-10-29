import { CheckCircle2, Handshake, XCircle } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { toast } from '@/components/ui/toast-store'
import { Overridable } from '@/kernel/modules/Overridable'
import type { RecordSlotProps } from '@/kernel/modules/slots'
import { useLeadActions } from './api'
import { isOpenLead, LEAD_STAGE, LIFECYCLE_STAGE_FIELD } from './constants'
import { ConvertLeadDialog } from './ConvertLeadDialog'
import { DisqualifyLeadDialog, type LeadTarget } from './DisqualifyLeadDialog'
import { lifecycleErrorMessage } from './errors'
import { leadTarget } from './lead-target'

export function LeadRecordActions({ record, readOnly }: RecordSlotProps) {
  const { t } = useTranslation('leads')
  const { hasPermission } = useSession()
  const { qualify } = useLeadActions()
  const [disqualifying, setDisqualifying] = useState<LeadTarget | null>(null)
  const [converting, setConverting] = useState<LeadTarget | null>(null)
  const stage = record[LIFECYCLE_STAGE_FIELD]

  if (readOnly || !isOpenLead(stage)) return null
  const lead = leadTarget(record)

  const runQualify = async () => {
    try {
      await qualify.mutateAsync(lead.id)
      notifySuccess(t('inbox.qualified', { name: lead.name }))
    } catch (error) {
      toast({ title: lifecycleErrorMessage(error, t), variant: 'destructive' })
    }
  }

  return (
    <div className="flex flex-wrap items-center gap-2">
      {stage === LEAD_STAGE && (
        <Button size="sm" variant="outline" disabled={qualify.isPending} onClick={() => void runQualify()}>
          <CheckCircle2 aria-hidden="true" />
          {t('actions.qualify')}
        </Button>
      )}
      <Button size="sm" variant="outline" onClick={() => setDisqualifying(lead)}>
        <XCircle aria-hidden="true" />
        {t('actions.disqualify')}
      </Button>
      {hasPermission('deals.create') && (
        <Button size="sm" variant="outline" onClick={() => setConverting(lead)}>
          <Handshake aria-hidden="true" />
          {t('actions.convert')}
        </Button>
      )}
      <DisqualifyLeadDialog lead={disqualifying} onOpenChange={(open) => !open && setDisqualifying(null)} />
      <Overridable
        id="leads.ConvertDialog"
        component={ConvertLeadDialog}
        lead={converting}
        onOpenChange={(open) => !open && setConverting(null)}
      />
    </div>
  )
}
