import { BarChart3, CheckCircle2, Handshake, Plus, Users, XCircle } from 'lucide-react'
import { useState, type MouseEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { toast } from '@/components/ui/toast-store'
import { type FilterNode, type RecordItem, type RecordListPreset, RecordFormDialog, RecordListView, recordDetailHref, useRecordList } from '@/sdk'
import { Overridable } from '@/kernel/modules/Overridable'
import { useLeadActions } from './api'
import { CONTACT_RECORD_TYPE, LEAD_STAGE, LEADS_REPORT_PATH, LIFECYCLE_STAGE_FIELD, QUALIFIED_STAGE } from './constants'
import { ConvertLeadDialog } from './ConvertLeadDialog'
import { DisqualifyLeadDialog, type LeadTarget } from './DisqualifyLeadDialog'
import { lifecycleErrorMessage } from './errors'
import { leadTarget } from './lead-target'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const PRESET_FILTERS: Record<'new' | 'qualified' | 'open', FilterNode> = {
  new: { field: LIFECYCLE_STAGE_FIELD, operator: 'eq', value: LEAD_STAGE },
  qualified: { field: LIFECYCLE_STAGE_FIELD, operator: 'eq', value: QUALIFIED_STAGE },
  open: { field: LIFECYCLE_STAGE_FIELD, operator: 'in', value: [LEAD_STAGE, QUALIFIED_STAGE] },
}
const PRESET_KEYS = ['new', 'qualified', 'open'] as const
const COLUMNS = ['name', 'email', 'phone', 'source', 'score', 'owner_id', 'created_at']

function usePresetCounts() {
  const news = useRecordList(CONTACT_RECORD_TYPE, { page: 1, perPage: 1, filter: PRESET_FILTERS.new })
  const qualified = useRecordList(CONTACT_RECORD_TYPE, { page: 1, perPage: 1, filter: PRESET_FILTERS.qualified })
  const open = useRecordList(CONTACT_RECORD_TYPE, { page: 1, perPage: 1, filter: PRESET_FILTERS.open })
  return { new: news.data?.meta.total, qualified: qualified.data?.meta.total, open: open.data?.meta.total }
}

export function LeadsPage() {
  const { t } = useTranslation('leads')
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const canUpdate = hasPermission('people.update')
  const canConvert = canUpdate && hasPermission('deals.create')
  const { qualify } = useLeadActions()
  const [disqualifying, setDisqualifying] = useState<LeadTarget | null>(null)
  const [converting, setConverting] = useState<LeadTarget | null>(null)
  const [creating, setCreating] = useState(false)
  const counts = usePresetCounts()
  const canCreate = hasPermission('people.create')
  const presets: RecordListPreset[] = PRESET_KEYS.map((key) => ({
    key,
    label: t(`inbox.presets.${key}`),
    filter: PRESET_FILTERS[key],
    emptyTitle: t(`inbox.empty.${key}.title`),
    emptyDescription: t(`inbox.empty.${key}.description`),
  }))
  const newButton = canCreate ? (
    <Button size="sm" onClick={() => setCreating(true)}>
      <Plus aria-hidden="true" />
      {t('inbox.new')}
    </Button>
  ) : undefined
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
    <Page>
      <PageHeader
        title={t('inbox.title')}
        description={t('inbox.description')}
        actions={
          <>
            <Button asChild size="sm" variant="outline">
              <Link to="/people">
                <Users aria-hidden="true" />
                {t('inbox.allContacts')}
              </Link>
            </Button>
            <Button asChild size="sm" variant="outline">
              <Link to={LEADS_REPORT_PATH}>
                <BarChart3 aria-hidden="true" />
                {t('report.open')}
              </Link>
            </Button>
            {newButton}
          </>
        }
      />
      <RecordListView
        recordType={CONTACT_RECORD_TYPE}
        presets={presets}
        presetCounts={counts}
        defaultColumns={COLUMNS}
        calendar={false}
        emptyAction={newButton}
        onRowClick={(record) => {
          const href = recordDetailHref('people.person', record.id)
          if (href) void navigate(href)
        }}
        titleAddon={actions}
      />
      {canCreate && (
        <RecordFormDialog
          recordType={CONTACT_RECORD_TYPE}
          open={creating}
          onOpenChange={setCreating}
          presetValues={{ [LIFECYCLE_STAGE_FIELD]: LEAD_STAGE }}
          title={t('inbox.newTitle')}
          description={t('inbox.newDescription')}
          onCreated={(record) => {
            const href = recordDetailHref('people.person', record.id)
            if (href) void navigate(href)
          }}
        />
      )}
      <DisqualifyLeadDialog lead={disqualifying} onOpenChange={(open) => !open && setDisqualifying(null)} />
      <Overridable
        id="leads.ConvertDialog"
        component={ConvertLeadDialog}
        lead={converting}
        onOpenChange={(open) => !open && setConverting(null)}
      />
    </Page>
  )
}
