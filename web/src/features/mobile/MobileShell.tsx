import { useQuery } from '@tanstack/react-query'
import { useState, type ReactNode } from 'react'
import { useLocation } from 'react-router'
import { recordsKeys } from '@/features/records/api'

import { recordDetailApi } from '@/features/records/detail/api'
import { getRecordResource, recordTitle } from '@/features/records/resources'
import { useModuleQuickAdds } from '@/kernel/modules/quick-add'
import { MobileNav } from './components/MobileNav'
import { QuickAddSheet, type QuickAddMode } from './components/QuickAddSheet'
import { MobileQuickAddContext } from './quick-add-context'
import type { PickedRecord } from './components/RecordPicker'
import { matchRecordRoute, type RecordRouteMatch } from './lib/record-route'

export interface MobileShellProps {
  onOpenMenu: () => void
  children?: ReactNode
}

function useRouteRecord(match: RecordRouteMatch | null): PickedRecord | null {
  const resource = match ? getRecordResource(match.recordType) : undefined
  const recordId = match?.recordId ?? ''
  const record = useQuery({
    queryKey: recordsKeys.detail(match?.recordType ?? '', recordId),
    queryFn: () => (resource ? recordDetailApi.get(resource, recordId) : Promise.reject(new Error('unknown record'))),
    enabled: Boolean(resource && recordId),
  })
  if (!match || !resource || !record.data) return null
  return { recordType: match.recordType, recordId: match.recordId, title: recordTitle(resource, record.data) }
}

export function MobileShell({ onOpenMenu, children }: MobileShellProps) {
  const actions = useModuleQuickAdds()
  const location = useLocation()
  const [mode, setMode] = useState<QuickAddMode | null>(null)
  const routeRecord = useRouteRecord(matchRecordRoute(location.pathname))
  const [sheetRecord, setSheetRecord] = useState<PickedRecord | null>(null)

  const open = (next: QuickAddMode) => {
    setSheetRecord(routeRecord)
    setMode(next)
  }

  return (
    <MobileQuickAddContext.Provider value={{ actions, open }}>
      {children}
      <MobileNav onOpenMenu={onOpenMenu} />
      <QuickAddSheet mode={mode} record={sheetRecord} actions={actions} onModeChange={setMode} />
    </MobileQuickAddContext.Provider>
  )
}
