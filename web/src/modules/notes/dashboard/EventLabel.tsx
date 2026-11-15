import { useTranslation } from 'react-i18next'
const ACTIONS = {'note.created':'created','note.updated':'updated','note.deleted':'deleted'} as const
export function NotesDashboardEventLabel({ event }: { event: { event_type: string; payload: Record<string, unknown> | null } }) {
  const fallback = useTranslation('dashboard').t('events.fallback')
  const { t } = useTranslation('notes', { keyPrefix: 'dashboard' })
  const action = ACTIONS[event.event_type as keyof typeof ACTIONS]
  return action ? t(`events.${action}`) : fallback
}
