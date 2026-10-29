import { BarChart3, CheckCircle2, Handshake, XCircle } from 'lucide-react'
import { useState, type MouseEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { toast } from '@/components/ui/toast-store'
import { RecordListView } from '@/features/records/table'
import { Overridable } from '@/kernel/modules/Overridable'
import type { RecordItem } from '@/features/records/types'
import { contactResource } from '@/modules/contacts/resource'
import { useLeadActions } from './api'
import { CONTACT_RECORD_TYPE, LEAD_INBOX_FILTER, LEADS_REPORT_PATH } from './constants'
import { ConvertLeadDialog } from './ConvertLeadDialog'
import { DisqualifyLeadDialog, type LeadTarget } from './DisqualifyLeadDialog'
import { lifecycleErrorMessage } from './errors'
import { leadTarget } from './lead-target'

export function LeadsPage() {
  const { t } = useTranslation('leads')
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const canUpdate = hasPermission('contacts.update')
  const canConvert = canUpdate && hasPermission('deals.create')
  const { qualify } = useLeadActions()
  const [disqualifying, setDisqualifying] = useState<LeadTarget | null>(null)
  const [converting, setConverting] = useState<LeadTarget | null>(null)
  const [busyId, setBusyId] = useState<string | null>(null)

  const runQualify = async (lead: LeadTarget) => {
    setBusyId(lead.id)
    try {
      await qualify.mutateAsync(lead.id)
      notifySuccess(t('inbox.qualified', { name: lead.name }))
    } catch (error) {
      toast({ title: lifecycleErrorMessage(error, t), variant: 'destructive' })
    } finally {
      setBusyId(null)
    }
  }

  const stop = (event: MouseEvent, action: () => void) => {
    event.stopPropagation()
    action()
  }

  const actions = (record: RecordItem) => {
    if (!canUpdate) return null
    const lead = leadTarget(record)
    return (
      <span className="ml-2 inline-flex items-center gap-1">
        <Button
          size="sm"
          variant="ghost"
          className="h-7 px-2"
          disabled={busyId === lead.id}
          title={t('actions.qualify')}
          aria-label={t('actions.qualifyLabel', { name: lead.name })}
          onClick={(event) => stop(event, () => void runQualify(lead))}
        >
          <CheckCircle2 aria-hidden="true" />
          <span className="hidden lg:inline">{t('actions.qualify')}</span>
        </Button>
        <Button
          size="sm"
          variant="ghost"
          className="h-7 px-2"
          title={t('actions.disqualify')}
          aria-label={t('actions.disqualifyLabel', { name: lead.name })}
          onClick={(event) => stop(event, () => setDisqualifying(lead))}
        >
          <XCircle aria-hidden="true" />
          <span className="hidden lg:inline">{t('actions.disqualify')}</span>
        </Button>
        {canConvert && (
          <Button
            size="sm"
            variant="ghost"
            className="h-7 px-2"
            title={t('actions.convert')}
            aria-label={t('actions.convertLabel', { name: lead.name })}
            onClick={(event) => stop(event, () => setConverting(lead))}
          >
            <Handshake aria-hidden="true" />
            <span className="hidden lg:inline">{t('actions.convert')}</span>
          </Button>
        )}
      </span>
    )
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="grid gap-1">
          <h1 className="text-xl font-semibold">{t('inbox.title')}</h1>
          <p className="text-sm text-muted-foreground">{t('inbox.description')}</p>
        </div>
        <Button asChild size="sm" variant="outline">
          <Link to={LEADS_REPORT_PATH}>
            <BarChart3 aria-hidden="true" />
            {t('report.open')}
          </Link>
        </Button>
      </div>
      <RecordListView
        recordType={CONTACT_RECORD_TYPE}
        baseFilter={LEAD_INBOX_FILTER}
        onRowClick={(record) => void navigate(contactResource.detailPath!(record.id))}
        titleAddon={actions}
      />
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
