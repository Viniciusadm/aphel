import { toast } from '@/components/ui/toast-store'
import { type RecordItem } from '@/sdk'
import i18n from '@/i18n'
import { navigateApp } from '@/kernel/modules/app-navigation'
import type { DuplicateMatch } from './api'
import { DUPLICATES_SETTINGS_PATH, mergePagePath } from './paths'

const MAX_NAMES = 3

export function possibleDuplicatesOf(record: RecordItem): DuplicateMatch[] {
  const matches = record.possible_duplicates
  return Array.isArray(matches) ? (matches as DuplicateMatch[]) : []
}

function duplicatesListPath(recordType: string): string {
  return `${DUPLICATES_SETTINGS_PATH}?${new URLSearchParams({ type: recordType }).toString()}`
}

export function warnPossibleDuplicates(recordType: string, record: RecordItem): number | undefined {
  const matches = possibleDuplicatesOf(record)
  if (matches.length === 0) return undefined
  const names = matches
    .slice(0, MAX_NAMES)
    .map((match) => match.title)
    .join(', ')
  const [single] = matches.length === 1 ? matches : []
  const target = single ? mergePagePath(recordType, record.id, single.record_id) : duplicatesListPath(recordType)
  return toast({
    title: i18n.t('warning.title', { ns: 'duplicates' }),
    description: i18n.t('warning.description', { ns: 'duplicates', count: matches.length, names }),
    duration: 10_000,
    action: {
      label: i18n.t(single ? 'warning.merge' : 'warning.review', { ns: 'duplicates' }),
      onClick: () => navigateApp(target),
    },
  })
}
