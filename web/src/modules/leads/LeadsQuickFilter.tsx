import { UserPlus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useLocation } from 'react-router'
import { Button } from '@/components/ui/button'
import { useRecordList } from '@/sdk'
import type { SlotProps } from '@/kernel/modules/slots'
import { CONTACT_RECORD_TYPE, LEAD_INBOX_FILTER, LEADS_PATH } from './constants'

function isLeadFilter(filter: SlotProps<'record.list.toolbar'>['filter']) {
  return JSON.stringify(filter) === JSON.stringify(LEAD_INBOX_FILTER)
}

export function LeadsQuickFilter({ filter, applyFilter }: SlotProps<'record.list.toolbar'>) {
  const { t } = useTranslation('leads')
  const { pathname } = useLocation()
  const active = isLeadFilter(filter)
  const count = useRecordList(CONTACT_RECORD_TYPE, { page: 1, perPage: 1, filter: LEAD_INBOX_FILTER }).data?.meta.total
  if (pathname === LEADS_PATH || pathname.startsWith(`${LEADS_PATH}/`)) return null
  return (
    <Button
      size="sm"
      variant={active ? 'secondary' : 'outline'}
      aria-pressed={active}
      title={t('quickFilter.description')}
      onClick={() => applyFilter(active ? null : LEAD_INBOX_FILTER)}
    >
      <UserPlus aria-hidden="true" />
      {t('quickFilter.label')}
      {count !== undefined && <span className="tabular-nums text-muted-foreground">({count})</span>}
    </Button>
  )
}
