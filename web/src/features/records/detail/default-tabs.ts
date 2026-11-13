import { CheckSquare, History, Link2, NotebookPen, Paperclip } from 'lucide-react'
import { createElement, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { AttachmentsPanel } from '@/features/files'
import { useIsModuleActive } from '@/kernel/modules/registry'
import { recordsKeys } from '../api'
import { Timeline } from '../timeline/Timeline'
import type { TimelineSource } from '../timeline/types'
import { RecordTabPlaceholder } from './RecordTabPlaceholder'
import type { RecordTab } from './types'

export interface DefaultRecordTabsOptions {
  recordType: string
  recordId: string
  readOnly?: boolean
  timeline?: TimelineSource
}

export function useDefaultRecordTabs({ recordType, recordId, readOnly = false, timeline }: DefaultRecordTabsOptions) {
  const { t } = useTranslation('records-detail')
  const notesActive = useIsModuleActive('notes')
  const tasksActive = useIsModuleActive('tasks')
  const timelineActive = useIsModuleActive('timeline')
  return useMemo<RecordTab[]>(
    () => [
      {
        key: 'timeline',
        group: 'activity',
        summary: timeline ? createElement(Timeline, { queryKey: recordsKeys.detail(recordType, recordId), fetchPage: timeline.fetchPage, autoLoad: false, limit: 5 }) : undefined,
        label: t('tabs.timeline'),
        icon: History,
        content: timeline
          ? createElement(Timeline, {
              queryKey: recordsKeys.detail(recordType, recordId),
              fetchPage: timeline.fetchPage,
              kindOptions: timeline.kindOptions,
              fetchKindOptions: timeline.fetchKindOptions,
            })
          : createElement(RecordTabPlaceholder, { icon: History, description: t('placeholders.timeline') }),
      },
      {
        key: 'tasks',
        group: 'activity',
        label: t('tabs.tasks'),
        icon: CheckSquare,
        content: createElement(RecordTabPlaceholder, { icon: CheckSquare, description: t('placeholders.tasks') }),
      },
      {
        key: 'notes',
        group: 'activity',
        label: t('tabs.notes'),
        icon: NotebookPen,
        content: createElement(RecordTabPlaceholder, { icon: NotebookPen, description: t('placeholders.notes') }),
      },
      {
        key: 'files',
        group: 'activity',
        label: t('tabs.files'),
        icon: Paperclip,
        content: createElement(AttachmentsPanel, { recordType, recordId, readOnly }),
      },
      {
        key: 'related',
        group: 'relationships',
        label: t('tabs.related'),
        icon: Link2,
        content: createElement(RecordTabPlaceholder, { icon: Link2, description: t('placeholders.related') }),
      },
    ].filter((tab) => (tab.key !== 'notes' || notesActive) && (tab.key !== 'tasks' || tasksActive) && (tab.key !== 'timeline' || timelineActive)),
    [t, recordType, recordId, readOnly, timeline, notesActive, tasksActive, timelineActive],
  )
}
