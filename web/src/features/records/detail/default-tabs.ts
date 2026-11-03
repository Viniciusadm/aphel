import { CheckSquare, History, Link2, NotebookPen, Paperclip } from 'lucide-react'
import { createElement, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { AttachmentsPanel } from '@/features/files'
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
  return useMemo<RecordTab[]>(
    () => [
      {
        key: 'timeline',
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
        label: t('tabs.tasks'),
        icon: CheckSquare,
        content: createElement(RecordTabPlaceholder, { icon: CheckSquare, description: t('placeholders.tasks') }),
      },
      {
        key: 'notes',
        label: t('tabs.notes'),
        icon: NotebookPen,
        content: createElement(RecordTabPlaceholder, { icon: NotebookPen, description: t('placeholders.notes') }),
      },
      {
        key: 'files',
        label: t('tabs.files'),
        icon: Paperclip,
        content: createElement(AttachmentsPanel, { recordType, recordId, readOnly }),
      },
      {
        key: 'related',
        label: t('tabs.related'),
        icon: Link2,
        content: createElement(RecordTabPlaceholder, { icon: Link2, description: t('placeholders.related') }),
      },
    ],
    [t, recordType, recordId, readOnly, timeline],
  )
}
