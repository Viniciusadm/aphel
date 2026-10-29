import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { useLocation } from 'react-router'
import { recordsKeys } from '@/features/records/api'
import { recordDetailApi } from '@/features/records/detail/api'
import { getRecordResource, recordTitle } from '@/features/records/resources'
import { useModuleQuickAdds } from '@/kernel/modules/quick-add'
import { MobileNav } from './components/MobileNav'
import { QUICK_ADD_MENU, QuickAddSheet, type QuickAddMode } from './components/QuickAddSheet'
import type { PickedRecord } from './components/RecordPicker'
import { matchRecordRoute, type RecordRouteMatch } from './lib/record-route'

export interface MobileShellProps {
  onOpenMenu: () => void
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

export function MobileShell({ onOpenMenu }: MobileShellProps) {
  const actions = useModuleQuickAdds()
  const shortcut = actions.find((action) => action.recordShortcut)
  const location = useLocation()
  const [mode, setMode] = useState<QuickAddMode | null>(null)
  const routeRecord = useRouteRecord(matchRecordRoute(location.pathname))
  const [sheetRecord, setSheetRecord] = useState<PickedRecord | null>(null)

  const open = (next: QuickAddMode) => {
    setSheetRecord(routeRecord)
    setMode(next)
  }

  return (
    <>
      {routeRecord && shortcut && (
        <button
          type="button"
          onClick={() => open(shortcut.id)}
          className="fixed right-4 bottom-[calc(4.5rem+env(safe-area-inset-bottom))] z-30 inline-flex h-11 items-center gap-2 rounded-full bg-primary-solid px-4 text-sm font-medium text-primary-foreground shadow-lg focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:outline-none md:hidden print:hidden"
        >
          <shortcut.icon className="size-4" aria-hidden="true" />
          {shortcut.recordShortcut}
        </button>
      )}
      <MobileNav onAdd={() => open(QUICK_ADD_MENU)} onOpenMenu={onOpenMenu} />
      <QuickAddSheet mode={mode} record={sheetRecord} actions={actions} onModeChange={setMode} />
    </>
  )
}
