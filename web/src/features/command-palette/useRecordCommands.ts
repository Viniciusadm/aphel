import { Link2, PanelTop, StickyNote } from 'lucide-react'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { toast } from '@/components/ui/toast-store'
import { RECORD_TAB_PARAM } from '@/features/records/detail/use-active-tab'
import { matchesRecordType } from '@/kernel/modules/record-types'
import { useCommandPalette, useRegisterCommands } from './context'
import { useCreateActions } from './create-actions'
import type { PaletteCommand } from './types'

const NOTES_TAB = 'notes'

export interface RecordCommandTab {
  key: string
  label: string
}

export interface RecordCommandsOptions {
  recordType: string
  recordId: string
  tabs: RecordCommandTab[]
}

export function useRecordCommands({ recordType, recordId, tabs }: RecordCommandsOptions) {
  const { t } = useTranslation('command-palette')
  const { hasPermission } = useSession()
  const { openCreate } = useCommandPalette()
  const createActions = useCreateActions()
  const [, setSearchParams] = useSearchParams()

  const commands = useMemo<PaletteCommand[]>(() => {
    const selectTab = (key: string) =>
      setSearchParams(
        (current) => {
          const next = new URLSearchParams(current)
          next.set(RECORD_TAB_PARAM, key)
          return next
        },
        { replace: true },
      )
    const list: PaletteCommand[] = []
    for (const action of createActions) {
      if (!action.targetRecordTypes || !matchesRecordType(action.targetRecordTypes, recordType)) continue
      list.push({
        id: `record:linked:${action.key}`,
        label: action.linkedLabel ?? action.label,
        group: 'context',
        icon: action.icon,
        keywords: [action.label],
        run: () => openCreate({ kind: action.key, target: { recordType, recordId } }),
      })
    }
    if (tabs.some((tab) => tab.key === NOTES_TAB) && hasPermission('notes.create')) {
      list.push({
        id: 'record:add-note',
        label: t('commands.addNote'),
        group: 'context',
        icon: StickyNote,
        run: () => selectTab(NOTES_TAB),
      })
    }
    list.push({
      id: 'record:copy-link',
      label: t('commands.copyLink'),
      group: 'context',
      icon: Link2,
      run: () => {
        void navigator.clipboard
          ?.writeText(window.location.href)
          .then(() => toast({ title: t('commands.linkCopied'), variant: 'success' }))
      },
    })
    for (const tab of tabs) {
      list.push({
        id: `record:tab:${tab.key}`,
        label: t('commands.openTab', { tab: tab.label }),
        group: 'context',
        icon: PanelTop,
        keywords: [tab.label],
        run: () => selectTab(tab.key),
      })
    }
    return list
  }, [recordType, recordId, tabs, hasPermission, t, openCreate, setSearchParams, createActions])

  useRegisterCommands(commands)
}
